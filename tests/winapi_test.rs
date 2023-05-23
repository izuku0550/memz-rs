use memz_rs::{
    convert_str::{ToPCSTRWrapper, ToPCWSTRWrapper},
    ntdll::{library::Library, ntdll_api::RtlAdjustPrivilegeFn},
    wrap_windows_api::{
        lstrcmp_w, wrap_close_handle, wrap_create_toolhelp32_snapshot, wrap_get_proc_address,
        wrap_get_process_image_filename_a, wrap_load_library_a, wrap_process32_next, Resolution, WinError,
    },
    LMEM_ZEROINIT,
};
use std::{mem::size_of, ptr, slice};
use windows::{
    core::{strlen, wcslen, PCSTR, PCWSTR},
    Win32::{
        Foundation::NTSTATUS,
        System::Diagnostics::ToolHelp::{Process32First, PROCESSENTRY32},
    },
};

#[test]
fn convert_str_to_pcstr() {
    let str = "Hello World";
    let pcstr = str.to_pcstr();

    // 변환된 값 확인
    assert_eq!(str, unsafe {
        String::from_utf8_lossy(slice::from_raw_parts(pcstr.0, strlen(PCSTR(pcstr.0))))
    });
}

#[test]
fn convert_str_to_pcwstr() {
    let str_value = "Hello, world!";
    let pcwstr_value = str_value.to_pcwstr();

    // 변환된 값 확인
    assert_eq!(str_value, unsafe {
        String::from_utf16_lossy(slice::from_raw_parts(
            pcwstr_value.0,
            wcslen(PCWSTR(pcwstr_value.0)),
        ))
    });
}

#[test]
fn resolution_new() {
    let resolution = Resolution::new();
    assert!(resolution.scrh > 0);
    assert!(resolution.scrw > 0);
}

#[test]
fn use_lstrcmp_w() {
    let str1 = "Hello, world!";
    let str2 = "Hello, world!";
    assert_eq!(lstrcmp_w(str1, str2), false);

    let str3 = "Goodbye, world!";
    assert_eq!(lstrcmp_w(str2, str3), true);
}

#[test]
fn load_library_a() {
    let result = wrap_load_library_a("ntdll.dll").unwrap_or(Default::default());
    assert_ne!(result.0, 0);
}

#[test]
fn get_proc_address() {
    let ntdll = wrap_load_library_a("ntdll.dll").unwrap();
    let result = wrap_get_proc_address(ntdll, "RtlAdjustPrivilege").unwrap();
    assert_ne!(result, ptr::null())
}

#[test]
fn get_proc() {
    let mut tmp1 = 0;
    let lib = Library::new("ntdll.dll");
    let proc: Option<RtlAdjustPrivilegeFn> = lib.get_proc("RtlAdjustPrivilege");

    let status = match proc {
        Some(rtl_adjust_privilege) => rtl_adjust_privilege(19, 1, 0, &mut tmp1),
        None => panic!("Failed GetProc RtlAdjustPrivilege"),
    };

    assert_eq!(status, NTSTATUS(0));
}

#[test]
fn get_process_image_filename_a() {
    let mut f_buf1: Vec<u8> = vec![LMEM_ZEROINIT; 512];
    let res = wrap_get_process_image_filename_a(&mut f_buf1);
    assert!(res.is_ok());
}

#[test]
fn create_toolhelp32_snapshot() {
    let result = wrap_create_toolhelp32_snapshot();
    assert!(result.is_ok());
}

#[test]
fn close_handle() -> Result<(), WinError> {
    let snapshot = wrap_create_toolhelp32_snapshot()?;
    let res = wrap_close_handle(snapshot);
    assert!(res.is_ok());
    Ok(())
}

#[test]
fn test_wrap_process32_next() {
    let snapshot = wrap_create_toolhelp32_snapshot().unwrap();
    let mut entry = PROCESSENTRY32 {
        dwSize: size_of::<PROCESSENTRY32> as u32,
        ..Default::default()
    };
    unsafe {
        Process32First(snapshot, &mut entry);
    }
    let result = wrap_process32_next(snapshot, &mut entry);
    assert!(result);
}

// static HOOK_CALLED: AtomicBool = AtomicBool::new(false);

// unsafe extern "system" fn hook_proc(_: i32, w_param: WPARAM, _: LPARAM) -> LRESULT {
//     HOOK_CALLED.store(true, Ordering::SeqCst);
//     CallNextHookEx(HHOOK::default(), 0, w_param, LPARAM(0))
// }

// #[test]
// fn test_hook() -> Result<(), ()> {
//     wrap_set_windows_hook_ex_a(WH_KEYBOARD_LL, Some(hook_proc), HMODULE(0), 0)?;
//     assert!(HOOK_CALLED.load(Ordering::SeqCst));
//     Ok(())
// }
