use crate::winapi_type::*;
use windows::Win32::Foundation::NTSTATUS;

pub type RtlAdjustPrivilegeFn = extern "system" fn(
    privileges: ULONG,
    enable: BOOLEAN,
    current_thread: BOOLEAN,
    enabled: PBOOLEAN,
) -> NTSTATUS;

pub type NtRaiseHardErrorFn = extern "system" fn(
    error_status: NTSTATUS,
    number_of_parameters: DWORD,
    unicode_string_parameter_mask: DWORD,
    parameters: DWORD,
    valid_response_option: DWORD,
    response: PDWORD
) -> NTSTATUS;
