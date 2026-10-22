//! Acquire ownership before opening user data; report failures without a console.
use std::io;

#[cfg(windows)]
fn local_data() -> io::Result<std::path::PathBuf> {
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
    };
    let mut raw = std::ptr::null_mut();
    if unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, std::ptr::null_mut(), &mut raw) }
        < 0
    {
        return Err(io::Error::other(
            "Windows LocalAppData known folder unavailable.",
        ));
    }
    unsafe {
        let mut length = 0;
        while *raw.add(length) != 0 {
            length += 1;
        }
        let result = String::from_utf16(std::slice::from_raw_parts(raw, length))
            .map(std::path::PathBuf::from)
            .map_err(io::Error::other);
        CoTaskMemFree(raw.cast());
        result
    }
}

pub struct InstanceLock {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}
impl InstanceLock {
    pub fn acquire(scope: &str) -> io::Result<Self> {
        #[cfg(windows)]
        {
            use sha2::{Digest, Sha256};
            use windows_sys::Win32::{Foundation::*, System::Threading::CreateMutexW};
            let local = local_data()?;
            let digest = Sha256::digest(format!(
                "{}|{scope}",
                local.to_string_lossy().to_lowercase()
            ));
            let name: Vec<u16> = format!("Local\\OBTAINER-{:x}\0", digest)
                .encode_utf16()
                .collect();
            let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
                unsafe {
                    CloseHandle(handle);
                }
                return Err(io::Error::other("OBTAINER is already running for this workspace. Switch to its existing window, or close it before reopening."));
            }
            Ok(Self { handle })
        }
        #[cfg(not(windows))]
        {
            let _ = scope;
            Ok(Self {})
        }
    }
}
impl Drop for InstanceLock {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
pub fn report_failure(error: &dyn std::fmt::Display) {
    let message = format!("OBTAINER could not start.\n\n{error}\n\nClose any existing OBTAINER window. Check that your application data folder is writable and that this version supports your database. Existing data has not been reset.");
    eprintln!("{message}");
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let message: Vec<u16> = message
            .replace('\0', " ")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let title: Vec<u16> = "OBTAINER — startup failed\0".encode_utf16().collect();
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                message.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR | MB_TASKMODAL,
            );
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn duplicate_workspace_is_denied_until_owner_exits() {
        let scope = format!(
            "test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        );
        let first = InstanceLock::acquire(&scope).unwrap();
        assert!(InstanceLock::acquire(&scope).is_err());
        let different = InstanceLock::acquire(&(scope.clone() + "-other")).unwrap();
        drop(first);
        let reopened = InstanceLock::acquire(&scope).unwrap();
        drop((different, reopened));
    }
}
