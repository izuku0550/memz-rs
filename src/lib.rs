pub const MEM_ZEROINIT: u8 = 0;
pub const LMEM_ZEROINIT: u16 = 0;
pub const GMEM_ZEROINIT: u16 = 0;

pub mod data;
pub mod ntdll;
pub mod payloads;
pub mod utils;

pub mod winapi_type {
    pub type DWORD = u32;
    pub type PDWORD = *mut u32;
    pub type PBYTE = *mut u8;
    pub type ULONG = u32;
    pub type BOOLEAN = u8;
    pub type PBOOLEAN = *mut u8;
}

pub mod convert_str {
    use windows::core::{HSTRING, PCSTR, PCWSTR};

    pub trait ToPCSTR {
        fn to_pcstr(&self) -> PCSTR;
    }

    impl ToPCSTR for &str {
        fn to_pcstr(&self) -> PCSTR {
            PCSTR(format!("{}\0", self).as_ptr())
        }
    }

    impl ToPCSTR for PCSTR {
        fn to_pcstr(&self) -> PCSTR {
            *self
        }
    }

    pub trait ToPCWSTR {
        fn to_pcwstr(&self) -> PCWSTR;
    }

    impl ToPCWSTR for HSTRING {
        fn to_pcwstr(&self) -> PCWSTR {
            PCWSTR(self.as_ptr())
        }
    }

    impl ToPCWSTR for PCWSTR {
        fn to_pcwstr(&self) -> PCWSTR {
            *self
        }
    }

    impl ToPCWSTR for &str {
        fn to_pcwstr(&self) -> PCWSTR {
            
            PCWSTR(HSTRING::from(&format!("{}\0", self)).as_ptr())
        }
    }

    impl ToPCWSTR for String {
        fn to_pcwstr(&self) -> PCWSTR {
            PCWSTR(HSTRING::from(&format!("{}\0", self)).as_ptr())
        }
    }

    #[macro_export]
    macro_rules! s_v {
        ($s:expr) => {
            windows::core::PCSTR::from_raw(format!("{}\0", $s).as_ptr())
        };
    }
}

pub mod wrap_windows_api {
    use crate::convert_str::{ToPCSTR, ToPCWSTR};
    #[cfg(feature = "DEBUG_MODE")]
    use crate::utils::log::*;
    use core::fmt;
    use std::{ffi::c_void, mem::size_of, process};
    use windows::{
        core::{HSTRING, PCWSTR},
        imp::{GetProcAddress, LoadLibraryA},
        Win32::{
            Foundation::{
                CloseHandle, GetLastError, BOOL, ERROR_NOT_ALL_ASSIGNED, GENERIC_ACCESS_RIGHTS,
                HANDLE, HMODULE, HWND, INVALID_HANDLE_VALUE, LUID,
            },
            Globalization::lstrcmpW,
            Security::{
                AdjustTokenPrivileges, LookupPrivilegeValueW, SECURITY_ATTRIBUTES,
                SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
                TOKEN_PRIVILEGES_ATTRIBUTES, TOKEN_QUERY,
            },
            Storage::FileSystem::{
                CreateFileA, FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE,
            },
            System::{
                Diagnostics::ToolHelp::{
                    CreateToolhelp32Snapshot, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
                },
                LibraryLoader::GetModuleFileNameW,
                ProcessStatus::GetProcessImageFileNameA,
                Threading::{
                    GetCurrentProcess, GetCurrentThreadId, OpenProcessToken, SetPriorityClass,
                    PROCESS_CREATION_FLAGS,
                },
            },
            UI::{
                Shell::ShellExecuteW,
                WindowsAndMessaging::{
                    GetMessageA, GetSystemMetrics, MessageBoxA, RegisterClassExA,
                    SetWindowsHookExA, UnhookWindowsHookEx, HHOOK, HOOKPROC, MESSAGEBOX_RESULT,
                    MESSAGEBOX_STYLE, MSG, SHOW_WINDOW_CMD, SYSTEM_METRICS_INDEX, WINDOWS_HOOK_ID,
                    WNDCLASSEXA,
                },
            },
        },
    };

    #[derive(Debug)]
    pub enum WinError {
        Failed,
        NoPermissions,
    }

    impl fmt::Display for WinError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    pub fn wrap_get_system_metrics(nindex: SYSTEM_METRICS_INDEX) -> Result<i32, WinError> {
        unsafe {
            match GetSystemMetrics(nindex) {
                // GetLastError() does not provide extended error
                0 => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        "Failed GetSystemMetrics function",
                    );
                    Err(WinError::Failed)
                }
                value => Ok(value),
            }
        }
    }

    pub fn lstrcmp_w<T, U>(str1: T, str2: U) -> bool
    where
        T: AsRef<HSTRING>,
        U: AsRef<HSTRING>,
    {
        unsafe {
            let str1 = str1.as_ref();
            let str2 = str2.as_ref();

            let cmp = lstrcmpW(str1, str2);

            cmp != 0
        }
    }

    pub fn wrap_get_process_image_filename_a(fn_buf: &mut [u8]) -> Result<u32, WinError> {
        unsafe {
            match GetProcessImageFileNameA(GetCurrentProcess(), fn_buf) {
                0 => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!("Failed to GetProcessImageFileNameA\n{:?}", GetLastError()),
                    );
                    Err(WinError::Failed)
                }
                v => Ok(v),
            }
        }
    }

    pub fn wrap_create_toolhelp32_snapshot() -> Result<HANDLE, WinError> {
        unsafe {
            match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
                Ok(INVALID_HANDLE_VALUE) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!("Failed CreateToolhelp32Snapshot\n{:?}", GetLastError()),
                    );
                    Err(WinError::Failed)
                }
                Ok(handle) => Ok(handle),
                _ => panic!(),
            }
        }
    }

    pub fn wrap_close_handle(h_object: HANDLE) -> Result<BOOL, WinError> {
        unsafe {
            match CloseHandle(h_object) {
                BOOL(0) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!("CloseHandle Error\n{:?}", GetLastError()),
                    );
                    Err(WinError::Failed)
                }
                ret => Ok(ret),
            }
        }
    }

    pub fn wrap_process32_next(hsnapshot: HANDLE, lppe: &mut PROCESSENTRY32) -> bool {
        unsafe {
            match Process32Next(hsnapshot, lppe) {
                BOOL(1) => true,
                BOOL(0) => false,
                _ => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::LOG,
                        "Failed Process32Next()\nUnknown Error",
                    );
                    panic!("Failed Process32Next()\nUnknown Error")
                }
            }
        }
    }

    pub fn wrap_set_windows_hook_ex_a(
        idhook: WINDOWS_HOOK_ID,
        lpfn: HOOKPROC,
        hmod: HMODULE,
        dwthreadid: u32,
    ) -> Result<HHOOK, WinError> {
        unsafe {
            match SetWindowsHookExA(idhook, lpfn, hmod, dwthreadid) {
                #[allow(unused_variables)]
                Err(e) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!(
                            "SetWindowsHookExA Error:\n{e:#?}\nGetLastError(): {:?}",
                            GetLastError()
                        ),
                    );
                    Err(WinError::Failed)
                }
                Ok(h_handle) => Ok(h_handle),
            }
        }
    }

    pub fn wrap_get_current_thread_id() -> u32 {
        unsafe { GetCurrentThreadId() }
    }

    pub fn wrap_messagebox_a<T, U>(
        hwnd: HWND,
        lptext: T,
        lpcaption: U,
        utype: MESSAGEBOX_STYLE,
    ) -> Result<MESSAGEBOX_RESULT, WinError>
    where
        T: ToPCSTR,
        U: ToPCSTR,
    {
        unsafe {
            match MessageBoxA(hwnd, lptext.to_pcstr(), lpcaption.to_pcstr(), utype) {
                MESSAGEBOX_RESULT(0) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!("MessageBoxA Error\nGetLastError(): {:?}", GetLastError()),
                    );
                    Err(WinError::Failed)
                }
                v => Ok(v),
            }
        }
    }

    pub fn wrap_unhook_windows_hook_ex(hhk: HHOOK) -> Result<bool, WinError> {
        unsafe {
            match UnhookWindowsHookEx(hhk) {
                BOOL(0) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!(
                            "UnhookWindowsHookEx Error\nGetLastError(): {:?}",
                            GetLastError()
                        ),
                    );
                    Err(WinError::Failed)
                }
                v => Ok(v.as_bool()),
            }
        }
    }

    pub fn wrap_load_library_a<T>(name: T) -> Result<HMODULE, WinError>
    where
        T: ToPCSTR,
    {
        unsafe {
            match LoadLibraryA(name.to_pcstr()) {
                0 => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!("Failed LoadLibraryA\nGetLastError(): {:?}", GetLastError()),
                    );
                    Err(WinError::Failed)
                }
                v => Ok(HMODULE(v)),
            }
        }
    }

    pub fn wrap_get_proc_address<T>(library: HMODULE, name: T) -> Result<*const c_void, WinError>
    where
        T: ToPCSTR,
    {
        unsafe {
            let ret = GetProcAddress(library.0, name.to_pcstr());
            if ret.is_null() {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::ALL,
                    &format!(
                        "Failed GetProcAddress\nGetLastError(): {:?}",
                        GetLastError()
                    ),
                );
                Err(WinError::Failed)
            } else {
                Ok(ret)
            }
        }
    }

    pub fn set_privilege<T>(lpsz_privilege: T, b_enabl_privilege: bool) -> Result<bool, WinError>
    where
        T: ToPCWSTR,
    {
        let mut h_token = HANDLE::default();
        let mut tp: TOKEN_PRIVILEGES = Default::default();
        let mut luid: LUID = Default::default();

        let lpv = unsafe { LookupPrivilegeValueW(PCWSTR::null(), lpsz_privilege.to_pcwstr(), &mut luid) };
        let opt = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut h_token,
            )
        };
        if !(lpv.as_bool() && opt.as_bool()) {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(
                LogType::ERROR,
                LogLocation::ALL,
                &format!(
                    "Failed LookupPrivilegeValueA()\nGetLastError: {:?}",
                    unsafe { GetLastError() }
                ),
            );
            return Err(WinError::Failed);
        }

        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = luid;

        if b_enabl_privilege {
            tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        } else {
            tp.Privileges[0].Attributes = TOKEN_PRIVILEGES_ATTRIBUTES(0);
        }

        let atp = unsafe {
            AdjustTokenPrivileges(
                h_token,
                BOOL(0),
                Some(&tp),
                size_of::<TOKEN_PRIVILEGES>() as u32,
                None,
                None,
            )
        };

        if !atp.as_bool() {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(
                LogType::ERROR,
                LogLocation::ALL,
                &format!(
                    "Failed AdjustTokenPrivileges()\nGetLastError: {:?}",
                    unsafe { GetLastError() }
                ),
            );

            return Err(WinError::Failed);
        }

        unsafe {
            if GetLastError() == ERROR_NOT_ALL_ASSIGNED {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::ALL,
                    "The token does not have the specified privilege.\n",
                );

                return Err(WinError::NoPermissions);
            }
        }
        #[cfg(feature = "DEBUG_MODE")]
        dbg!(&tp);

        Ok(true)
    }

    pub fn wrap_register_class_ex_a(param0: &WNDCLASSEXA) -> Result<u16, WinError> {
        unsafe {
            match RegisterClassExA(param0) {
                0 => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!(
                            "Failed RegisterClassExA()\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );
                    Err(WinError::Failed)
                }
                v => Ok(v),
            }
        }
    }

    pub fn wrap_get_message(
        lpmsg: &mut MSG,
        hwnd: HWND,
        wmsgfiltermin: u32,
        wmsgfiltermax: u32,
    ) -> Result<bool, WinError> {
        unsafe {
            match GetMessageA(lpmsg, hwnd, wmsgfiltermin, wmsgfiltermax) {
                BOOL(-1) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!("Failed GetMessageA()\nGetLastError: {:?}", GetLastError()),
                    );

                    Err(WinError::Failed)
                }
                v => Ok(v.as_bool()),
            }
        }
    }

    pub fn wrap_get_module_file_name(
        hmodule: HMODULE,
        lpfilename: &mut [u16],
    ) -> Result<u32, WinError> {
        unsafe {
            match GetModuleFileNameW(hmodule, lpfilename) {
                0 => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!(
                            "Failed GetModuleFileNameExA()\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );
                    Err(WinError::Failed)
                }
                v => Ok(v),
            }
        }
    }

    pub fn wrap_shell_execute_w<P1, P2, P3, P4>(
        hwnd: HWND,
        lpoperation: P1,
        lpfile: P2,
        lpparameters: P3,
        lpdirectory: P4,
        nshowcmd: SHOW_WINDOW_CMD,
    ) -> Result<HMODULE, WinError>
    where
        P1: ToPCWSTR,
        P2: ToPCWSTR,
        P3: ToPCWSTR,
        P4: ToPCWSTR,
    {
        unsafe {
            let res = ShellExecuteW(
                hwnd,
                lpoperation.to_pcwstr(),
                lpfile.to_pcwstr(),
                lpparameters.to_pcwstr(),
                lpdirectory.to_pcwstr(),
                nshowcmd,
            );

            match res {
                HMODULE(0..=31) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!("ShellExecuteW failed with error code: {:?}", res),
                    );
                    Err(WinError::Failed)
                }
                v => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(LogType::INFO, LogLocation::ALL, "ShellExecuteW successed");
                    Ok(v)
                }
            }
        }
    }

    pub fn wrap_set_priority_class(
        h_process: HANDLE,
        dw_priority_class: u32,
    ) -> Result<bool, WinError> {
        unsafe {
            match SetPriorityClass(h_process, PROCESS_CREATION_FLAGS(dw_priority_class)) {
                BOOL(0) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!(
                            "SetPriorityClass failed with GetLastError():\n: {:?}",
                            GetLastError()
                        ),
                    );
                    Err(WinError::Failed)
                }
                v => Ok(v.as_bool()),
            }
        }
    }

    pub fn wrap_create_file_a<T>(
        lpfilename: T,
        dwdesiredaccess: GENERIC_ACCESS_RIGHTS,
        dwsharemode: FILE_SHARE_MODE,
        lpsecurityattributes: Option<*const SECURITY_ATTRIBUTES>,
        dwcreationdisposition: FILE_CREATION_DISPOSITION,
        dwflagsandattributes: FILE_FLAGS_AND_ATTRIBUTES,
        htemplatefile: HANDLE,
    ) -> Option<HANDLE>
    where
        T: ToPCSTR,
    {
        unsafe {
            match CreateFileA(
                lpfilename.to_pcstr(),
                dwdesiredaccess.0,
                dwsharemode,
                lpsecurityattributes,
                dwcreationdisposition,
                dwflagsandattributes,
                htemplatefile,
            ) {
                Ok(INVALID_HANDLE_VALUE) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!(
                            "Failed CreateFileA()\nError: INVALID_HANDLE_VALUE\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );
                    process::exit(2)
                }
                Ok(handle) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(LogType::INFO, LogLocation::ALL, "CreateFileA successed");
                    Some(handle)
                }
                #[allow(unused_variables)]
                Err(e) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!(
                            "Failed CreateFileA()\nError: {e:?}\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );
                    None
                }
            }
        }
    }
}
