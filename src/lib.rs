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
    // https://github.com/microsoft/windows-rs/issues/973#issuecomment-1363481060
    use windows::core::{PCSTR, PCWSTR};

    pub struct PCWSTRWrapper {
        text: PCWSTR,
        // this is here to allow it to get dropped at the same time as the PCWSTR
        #[allow(unused)]
        _container: Vec<u16>,
    }

    impl std::ops::Deref for PCWSTRWrapper {
        type Target = PCWSTR;

        fn deref(&self) -> &Self::Target {
            &self.text
        }
    }

    pub trait ToPCWSTRWrapper {
        fn to_pcwstr(&self) -> PCWSTRWrapper;
    }

    impl ToPCWSTRWrapper for &str {
        fn to_pcwstr(&self) -> PCWSTRWrapper {
            // do not drop when scope ends, by moving it into struct
            let mut text = self.encode_utf16().collect::<Vec<_>>();
            text.push(0);

            PCWSTRWrapper {
                text: PCWSTR::from_raw(text.as_ptr()),
                _container: text,
            }
        }
    }

    impl ToPCWSTRWrapper for PCWSTR {
        fn to_pcwstr(&self) -> PCWSTRWrapper {
            PCWSTRWrapper {
                text: *self,
                _container: Vec::new(),
            }
        }
    }

    pub struct PCSTRWrapper {
        text: PCSTR,
        #[allow(unused)]
        _container: Vec<u8>,
    }

    impl std::ops::Deref for PCSTRWrapper {
        type Target = PCSTR;

        fn deref(&self) -> &Self::Target {
            &self.text
        }
    }

    pub trait ToPCSTRWrapper {
        fn to_pcstr(&self) -> PCSTRWrapper;
    }

    impl ToPCSTRWrapper for &str {
        fn to_pcstr(&self) -> PCSTRWrapper {
            // https://stackoverflow.com/questions/47980023/how-to-convert-from-u8-to-vecu8
            let mut text = self.as_bytes().to_vec();
            text.push(0); // add null

            PCSTRWrapper {
                text: PCSTR(text.as_ptr()),
                _container: text, // data lifetime management
            }
        }
    }

    impl ToPCSTRWrapper for PCSTR {
        fn to_pcstr(&self) -> PCSTRWrapper {
            PCSTRWrapper {
                text: *self,
                _container: Vec::new(),
            }
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
    use core::fmt;
    use std::{ffi::c_void, mem::size_of, process};

    use crate::convert_str::{ToPCSTRWrapper, ToPCWSTRWrapper};
    use crate::utils::log::{write_log, LogLocation, LogType};
    use windows::{
        core::PCWSTR,
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
                ProcessStatus::GetProcessImageFileNameA,
                Threading::{
                    GetCurrentProcess, GetCurrentThreadId, OpenProcessToken, SetPriorityClass,
                    PROCESS_CREATION_FLAGS,
                },
                LibraryLoader::GetModuleFileNameW
            },
            UI::{
                WindowsAndMessaging::{
                    GetMessageA, GetSystemMetrics, MessageBoxA, RegisterClassExA,
                    SetWindowsHookExA, UnhookWindowsHookEx, HHOOK, HOOKPROC, MESSAGEBOX_RESULT,
                    MESSAGEBOX_STYLE, MSG, SHOW_WINDOW_CMD, SYSTEM_METRICS_INDEX, WINDOWS_HOOK_ID,
                    WNDCLASSEXA, LoadIconA, HICON
                },
                Shell::ShellExecuteW
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
        T: ToPCWSTRWrapper,
        U: ToPCWSTRWrapper,
    {
        unsafe {
            let str1 = str1.to_pcwstr();
            let str2 = str2.to_pcwstr();

            let cmp = lstrcmpW(*str1, *str2);

            cmp != 0
        }
    }

    pub fn wrap_get_process_image_filename_a(fn_buf: &mut [u8]) -> Result<u32, WinError> {
        unsafe {
            match GetProcessImageFileNameA(GetCurrentProcess(), fn_buf) {
                0 => {
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
                Err(e) => {
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
        T: ToPCSTRWrapper,
        U: ToPCSTRWrapper,
    {
        let lptext = *lptext.to_pcstr();
        let lpcaption = *lpcaption.to_pcstr();
        unsafe {
            match MessageBoxA(hwnd, lptext, lpcaption, utype) {
                MESSAGEBOX_RESULT(0) => {
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
        T: ToPCSTRWrapper,
    {
        let name = *name.to_pcstr();
        unsafe {
            match LoadLibraryA(name) {
                0 => {
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
        T: ToPCSTRWrapper,
    {
        let name = *name.to_pcstr();
        unsafe {
            let ret = GetProcAddress(library.0, name);
            if ret.is_null() {
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
        T: ToPCWSTRWrapper,
    {
        let lpsz_privilege = *lpsz_privilege.to_pcwstr();

        let mut h_token = HANDLE::default();
        let mut tp: TOKEN_PRIVILEGES = Default::default();
        let mut luid: LUID = Default::default();

        let lpv = unsafe { LookupPrivilegeValueW(PCWSTR::null(), lpsz_privilege, &mut luid) };
        let opt = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut h_token,
            )
        };
        if !(lpv.as_bool() && opt.as_bool()) {
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
            #[cfg(not(feature = "DEBUG_MODE"))]
            write_log(
                LogType::ERROR,
                LogLocation::MSG,
                &format!(
                    "Failed AdjustTokenPrivileges()\nGetLastError: {:?}",
                    unsafe { GetLastError() }
                ),
            );

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
                #[cfg(not(feature = "DEBUG_MODE"))]
                write_log(
                    LogType::ERROR,
                    LogLocation::MSG,
                    "The token does not have the specified privilege.\n",
                );

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
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    write_log(
                        LogType::ERROR,
                        LogLocation::MSG,
                        &format!(
                            "Failed RegisterClassExA()\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );
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
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    write_log(
                        LogType::ERROR,
                        LogLocation::MSG,
                        &format!("Failed GetMessageA()\nGetLastError: {:?}", GetLastError()),
                    );

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
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    write_log(
                        LogType::ERROR,
                        LogLocation::MSG,
                        &format!(
                            "Failed GetModuleFileNameExA()\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );
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
        P1: ToPCWSTRWrapper,
        P2: ToPCWSTRWrapper,
        P3: ToPCWSTRWrapper,
        P4: ToPCWSTRWrapper,
    {
        let p1 = *lpoperation.to_pcwstr();
        let p2 = *lpfile.to_pcwstr();
        let p3 = *lpparameters.to_pcwstr();
        let p4 = *lpdirectory.to_pcwstr();
        unsafe {
            let res = ShellExecuteW(hwnd, p1, p2, p3, p4, nshowcmd);

            match res {
                HMODULE(0..=31) => {
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    write_log(
                        LogType::ERROR,
                        LogLocation::MSG,
                        &format!("ShellExecuteW failed with error code: {:?}", res),
                    );
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
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    write_log(
                        LogType::ERROR,
                        LogLocation::MSG,
                        &format!(
                            "SetPriorityClass failed with GetLastError():\n: {:?}",
                            GetLastError()
                        ),
                    );
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
        T: ToPCSTRWrapper,
    {
        let lpfilename = *lpfilename.to_pcstr();
        unsafe {
            match CreateFileA(
                lpfilename,
                dwdesiredaccess.0,
                dwsharemode,
                lpsecurityattributes,
                dwcreationdisposition,
                dwflagsandattributes,
                htemplatefile,
            ) {
                Ok(INVALID_HANDLE_VALUE) => {
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    write_log(
                        LogType::ERROR,
                        LogLocation::MSG,
                        &format!(
                            "Failed CreateFileA()\nError: INVALID_HANDLE_VALUE\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );

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
                Err(e) => {
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    write_log(
                        LogType::ERROR,
                        LogLocation::MSG,
                        &format!(
                            "Failed CreateFileA()\nError: {e:?}\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );

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

    pub fn wrap_load_icon_a<T>(hinstance: HMODULE, lpiconname: T) -> windows::core::Result<HICON>
    where
        T: ToPCSTRWrapper,
    {
        let lpiconname = *lpiconname.to_pcstr();
        unsafe {
            match LoadIconA(hinstance, lpiconname) {
                Ok(v) => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(LogType::INFO, LogLocation::ALL, "SUCCESS LoadIconA()");
                    Ok(v)
                }
                Err(e) => {
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    write_log(
                        LogType::ERROR,
                        LogLocation::MSG,
                        &format!(
                            "Failed LoadIconA()\n{e}\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        &format!(
                            "Failed LoadIconA()\n{e}\nGetLastError: {:?}",
                            GetLastError()
                        ),
                    );
                    Err(e)
                }
            }
        }
    }
}
