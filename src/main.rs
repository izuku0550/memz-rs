#![allow(unused_imports)]
#![allow(dead_code)]
use memz_rs::{
    wrap_windows_api::{lstrcmp_w, wrap_get_process_image_filename_a, Commandline, Resolution},
    LMEM_ZEROINIT,
};
use std::{slice, thread::{self, sleep}, time::Duration};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::HWND, Globalization::lstrcmpW, Graphics::Gdi::HFONT,
        System::Environment::GetCommandLineW, UI::Shell::CommandLineToArgvW,
    },
};

// #[cfg(feature="CLEAN")]
struct Clean {
    main_window: HWND,
    font: HFONT,
    dialog: HWND,
}

fn main() -> windows::core::Result<()> {
    let res = Resolution::new();
    println!("{} {}", res.scrw, res.scrh);

    // #[cfg(feature="CLEAN")]
    {
        let cmdline = Commandline::new();
        let argc = cmdline.argc;
        let arg = cmdline.arg;

        if argc > 1 {
            if lstrcmp_w(arg, "/watchdog") {
                let watchdog_thread = thread::spawn(move || {
                    let oproc = 0;
                    let mut f_buf = vec![LMEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
                    if let Ok(v) = wrap_get_process_image_filename_a(&mut f_buf) {
                        v
                    } else {
                        0
                    };

                    sleep(Duration::from_nanos(1000));

                    loop {
                            let snapshot = todo!();
                    }
                });
                watchdog_thread.join().unwrap();
            }
        }
    }

    Ok(())
}
