pub const LMEM_ZEROINIT: u8 = 0;

pub mod memz {}

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
    use crate::convert_str::ToPCWSTRWrapper;
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{CloseHandle, GetLastError, BOOL, HANDLE, INVALID_HANDLE_VALUE},
            Globalization::lstrcmpW,
            System::{
                Diagnostics::ToolHelp::{
                    CreateToolhelp32Snapshot, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
                },
                Environment::GetCommandLineW,
                ProcessStatus::GetProcessImageFileNameA,
                Threading::GetCurrentProcess,
            },
            UI::{
                Shell::CommandLineToArgvW,
                WindowsAndMessaging::{
                    GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SYSTEM_METRICS_INDEX,
                },
            },
        },
    };

    pub struct Commandline {
        pub arg: PCWSTR,
        pub argc: i32,
    }

    impl Commandline {
        pub fn new() -> Self {
            Self {
                arg: Self::wrapping_get_args_w().unwrap().arg,
                argc: Self::wrapping_get_args_w().unwrap().argc,
            }
        }

        fn wrapping_get_args_w() -> Result<Commandline, ()> {
            unsafe {
                let mut argc = 0;
                let argv = CommandLineToArgvW(GetCommandLineW(), &mut argc);

                if !argv.is_null() {
                    let arg = *argv;
                    Ok(Commandline {
                        arg: PCWSTR(arg.0),
                        argc: argc,
                    })
                } else {
                    Err(eprintln!("Commandline Error\n{:?}", GetLastError()))
                }
            }
        }
    }

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

    pub fn wrap_process32_next(hsnapshot: HANDLE, lppe: &mut PROCESSENTRY32) -> Result<bool, ()> {
        unsafe {
            match Process32Next(hsnapshot, lppe) {
                BOOL(1) => Ok(true),
                BOOL(0) => Err(eprintln!("Process32Next Error\n{:?}", GetLastError())),
                _ => panic!(),
            }
        }
    }
}
