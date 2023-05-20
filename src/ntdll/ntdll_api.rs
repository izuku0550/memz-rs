use crate::{
    winapi_type::*,
    wrap_windows_api::{wrap_get_proc_address, wrap_load_library_a},
};
use windows::Win32::Foundation::{BOOL, NTSTATUS};

type RtlAdjustPrivilegeFn = unsafe extern "system" fn(
    Privilege: DWORD,
    Enable: BOOL,
    CurrentThread: BOOL,
    Enabled: PBYTE,
) -> NTSTATUS;

type NtRaiseHardErrorFn = unsafe extern "system" fn(
    ErrorStatus: NTSTATUS,
    NumberOfParameters: DWORD,
    UnicodeStringParameterMask: DWORD,
    Parameters: DWORD,
    ValidResponseOption: DWORD,
    Response: PDWORD,
) -> NTSTATUS;

pub fn wrap_rtladjustprivilege(
    privilege: DWORD,
    enable: bool,
    curreent_thread: bool,
    enabled: *mut u8,
) -> Result<NTSTATUS, ()> {
    if enabled.is_null() {
        return Err(eprintln!(
            "NTDLL Error: RtlAdjustPrivilege()\nenabled variable is null pointer"
        ));
    }

    let ntdll = wrap_load_library_a("ntdll.dll")?;
    dbg!(&ntdll);

    let rtl_adjust_privilege_fn =
        dbg!(wrap_get_proc_address(ntdll, "RtlAdjustPrivilege")?) as *const RtlAdjustPrivilegeFn;

    let status: NTSTATUS;

    if !rtl_adjust_privilege_fn.is_null() {
        unsafe {
            status = (*rtl_adjust_privilege_fn)(
                privilege,
                BOOL(enable as i32),
                BOOL(curreent_thread as i32),
                enabled,
            );
        }
        match status {
            NTSTATUS(0x00000000) => Ok(status),
            e => Err(eprintln!(
                "NTDLL Error: RtlAdjustPrivilege()\nError code: {e:?}\n"
            )),
        }
    } else {
        return Err(eprintln!(
            "NTDLL Error: RtlAdjustPrivilege() is null pointer"
        ));
    }
}

pub fn wrap_nt_raise_hard_error(
    error_status: NTSTATUS,
    number_of_parameters: DWORD,
    unicode_string_parameter_mask: DWORD,
    parameters: DWORD,
    valid_response_option: DWORD,
    response: PDWORD,
) -> Result<NTSTATUS, ()> {
    let ntdll = wrap_load_library_a("ntdll.dll")?;
    dbg!(&ntdll);

    let nt_raise_hard_error_fn =
        dbg!(wrap_get_proc_address(ntdll, "NtRaiseHardError")?) as *const NtRaiseHardErrorFn;

    let status: NTSTATUS;
    if !nt_raise_hard_error_fn.is_null() {
        unsafe {
            status = (*nt_raise_hard_error_fn)(
                error_status,
                number_of_parameters,
                unicode_string_parameter_mask,
                parameters,
                valid_response_option,
                response,
            );
        }
        match status {
            NTSTATUS(0x00000000) => Ok(status),
            e => Err(eprintln!(
                "NTDLL Error: NtRaiseHardError()\nError code: {e:?}"
            )),
        }
    } else {
        return Err(eprintln!("NTDLL Error: NtRaiseHardError() is null pointer"));
    }
}
