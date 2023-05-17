pub const LMEM_ZEROINIT: u8 = 0;

pub mod memz {}

pub mod convert_str {
    // https://github.com/microsoft/windows-rs/issues/973#issuecomment-1363481060
    use windows::core::PCWSTR;

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
}

pub mod wrap_windows_api {
    use crate::convert_str::ToPCWSTRWrapper;
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::GetLastError,
            Globalization::lstrcmpW,
            System::{ProcessStatus::GetProcessImageFileNameA, Threading::GetCurrentProcess, Environment::GetCommandLineW},
            UI::{WindowsAndMessaging::{
                GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SYSTEM_METRICS_INDEX,
            }, Shell::CommandLineToArgvW},
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
                    return Ok(Commandline {
                        arg: PCWSTR(arg.0),
                        argc: argc,
                    });
                } else {
                    return Err(eprintln!("Commandline Error\n{:?}", GetLastError()));
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
                    0 => return Err(eprintln!("Failed GetSystemMetrics function")),
                    value => return Ok(value),
                };
            }
        }
    }

    pub fn lstrcmp_w<T>(str1: PCWSTR, str2: T) -> bool
    where
        T: ToPCWSTRWrapper + AsRef<str>,
    {
        unsafe {
            let str2 = str2.to_pcwstr();
            let cmp = lstrcmpW(str1, *str2);

            if cmp == 0 {
                return false;
            } else if cmp < 0 {
                return true;
            } else {
                return true;
            }
        }
    }

    pub fn wrap_get_process_image_filename_a(fn_buf: &mut Vec<u8>) -> Result<u32, ()> {
        unsafe {
            match GetProcessImageFileNameA(GetCurrentProcess(), fn_buf) {
                0 => {
                    return Err(eprintln!(
                        "Failed to GetProcessImageFileNameA\n{:?}",
                        GetLastError()
                    ))
                }
                v => return Ok(v),
            };
        }
    }
}
