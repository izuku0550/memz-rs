pub mod memz {}

pub mod get_cmdline_to_argv_w {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::GetLastError, System::Environment::GetCommandLineW,
            UI::Shell::CommandLineToArgvW,
        },
    };
    pub struct Commandline {
        pub arg: PCWSTR,
        pub argc: i32
    }

    impl Commandline {
        pub fn new() -> Self {
            Self { 
                arg: Self::wrapping_get_args_w().unwrap().0,
                argc: Self::wrapping_get_args_w().unwrap().1
            }
        }

        fn wrapping_get_args_w() -> Result<(PCWSTR, i32), ()>{
            unsafe {
                let mut argc = 0;
                let argv = CommandLineToArgvW(GetCommandLineW(), &mut argc);

                if !argv.is_null() {
                    let arg = *argv;
                    return Ok((PCWSTR(arg.0), argc))
                } else {
                    return Err(eprintln!("Commandline Error: {:?}", GetLastError()))
                }
            }
        }
    }
}

pub mod screen {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SYSTEM_METRICS_INDEX,
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
                    0 => return Err(eprintln!("Failed GetSystemMetrics function")),
                    value => return Ok(value),
                };
            }
        }
    }
}

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

pub mod strcmp {
    use windows::{core::PCWSTR, Win32::Globalization::lstrcmpW};
    use crate::convert_str::ToPCWSTRWrapper;

    pub fn lstrcmp_w(str1: PCWSTR, str2: &str) -> bool {
        unsafe {
            let wrap_str = str2.to_pcwstr();
            let cmp = lstrcmpW(str1, *wrap_str);

            if cmp == 0 {
                return false
            } else if cmp < 0 {
                return true
            } else {
                return true
            }
        }
    }
}