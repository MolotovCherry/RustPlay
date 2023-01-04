use lazy_static::lazy_static;
use windows::Win32::{Foundation::NTSTATUS, System::SystemInformation::OSVERSIONINFOW};
use windows_dll::dll;

#[dll("ntdll.dll")]
extern "system" {
    #[allow(non_snake_case)]
    fn RtlGetVersion(lpVersionInformation: *mut OSVERSIONINFOW) -> NTSTATUS;
}

lazy_static! {
    static ref WINVER: u32 = {
        let mut version_info = OSVERSIONINFOW::default();
        unsafe {
            if RtlGetVersion(&mut version_info).is_err() {
                panic!("Failed to get version");
            }
        }

        version_info.dwBuildNumber
    };
}

#[inline]
pub fn is_win10_1809() -> bool {
    *WINVER >= 17763 && *WINVER < 22000
}

#[inline]
pub fn is_win11() -> bool {
    *WINVER >= 22000
}

#[inline]
pub fn is_win11_22h2() -> bool {
    *WINVER >= 22621
}

#[inline]
pub fn is_supported_os() -> bool {
    is_win10_1809() || is_win11()
}
