use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, CBT_CREATEWNDA, HCBT_CREATEWND, HHOOK, SM_CXSCREEN, SM_CYSCREEN,
        WINDOW_STYLE, WS_DLGFRAME, WS_POPUP,
    },
};

use crate::wrap_windows_api::wrap_get_system_metrics;

/// # Safety
///
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
