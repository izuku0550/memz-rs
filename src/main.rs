#![allow(unused_imports)]
use screen::Resolution;
use windows::Win32::{Foundation::HWND, Graphics::Gdi::HFONT};
mod screen {
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
                scrw: Self::wrap_with_option(SM_CXSCREEN).unwrap(),
                scrh: Self::wrap_with_option(SM_CYSCREEN).unwrap(),
            }
        }

        fn wrap_with_option(nindex: SYSTEM_METRICS_INDEX) -> Result<i32, ()> {
            unsafe {
                match GetSystemMetrics(nindex) {
                    0 => panic!("Failed GetSystemMetrics function"),
                    value => return Ok(value),
                };
            }
        }
    }
}

#[cfg(some_symbol)]
struct Clean {
    main_window: HWND,
    font: HFONT,
    dialog: HWND,
}

fn main() {
    let res = Resolution::new();
    println!("{} {}", res.scrw, res.scrh);

    #[cfg(some_symbol)]
    unsafe {

    }
}
