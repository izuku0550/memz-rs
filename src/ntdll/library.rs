use crate::wrap_windows_api::{wrap_get_proc_address, wrap_load_library_a};
use std::mem::transmute_copy;
use windows::Win32::Foundation::HMODULE;

pub enum NtdllError {
    NtdllError,
}

pub struct Library {
    handle: HMODULE,
}

impl Library {
    pub fn new(name: &str) -> Self {
        let res = wrap_load_library_a(name).expect("Failed LoadLibraryA()\nUnknown Error");
        Self { handle: res }
    }

    pub fn get_proc<T>(&self, name: &str) -> Option<T> {
        let res = wrap_get_proc_address(self.handle, name)
            .expect("Failed GetProcAddress()\nUnknown Error");
        unsafe { transmute_copy(&res) }
    }
}
