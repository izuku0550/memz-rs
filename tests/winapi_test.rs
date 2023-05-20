use memz_rs::{
    convert_str::{ToPCSTRWrapper, ToPCWSTRWrapper},
    ntdll::{data::Privilege, ntdll_api::wrap_rtladjustprivilege},
    wrap_windows_api::{
        lstrcmp_w, wrap_get_proc_address, wrap_load_library_a, Resolution,
    },
};
use std::{ptr, slice};
use windows::{
    core::{strlen, wcslen, PCSTR, PCWSTR},
    Win32::Foundation::{BOOL, NTSTATUS},
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
fn test_rtl_adjust_privilege() -> Result<(), ()> {
    let mut tmp1: bool = Default::default();
    let tmp1_ptr = &mut tmp1 as *mut bool as *mut u8;

    let status =
        wrap_rtladjustprivilege(Privilege::SeShutdownPrivilege as u32, true, false, tmp1_ptr)?;

    assert_eq!(status, NTSTATUS(0));
    Ok(())
}
