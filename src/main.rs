#![allow(unused_imports)]
#![allow(dead_code)]
use memz_rs::{get_cmdline_to_argv_w::Commandline, screen::Resolution, strcmp::lstrcmp_w};
use std::slice;
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

fn main() {
    let res = Resolution::new();
    println!("{} {}", res.scrw, res.scrh);

    // #[cfg(feature="CLEAN")]
    {
        let cmdline = Commandline::new();
        let argc = cmdline.argc;
        let arg = cmdline.arg;

        if argc > 1 {
            if lstrcmp_w(arg, "/watchdog") {
                todo!()
            }
        }
    }
}
