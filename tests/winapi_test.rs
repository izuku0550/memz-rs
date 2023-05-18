use memz_rs::{
    convert_str::{ToPCSTRWrapper, ToPCWSTRWrapper},
    wrap_windows_api::{Commandline, Resolution, lstrcmp_w},
};
use std::slice;
use windows::core::{strlen, wcslen, PCSTR, PCWSTR};

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
fn commandline_new() {
    let commandline = Commandline::new();
    assert!(commandline.argc > 0);
    assert!(!commandline.arg.0.is_null());
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

