use super::types::*;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
pub const CHUNK: usize = 1024 * 1024;
pub fn nonce() -> Result<String> {
    let mut bytes = [0u8; 16];
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Security::Cryptography::*;
        if BCryptGenRandom(
            std::ptr::null_mut(),
            bytes.as_mut_ptr(),
            16,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        ) < 0
        {
            return Err(LibraryError::new(
                "RANDOM_FAILED",
                "Windows could not create a secure operation ID.",
            ));
        }
    }
    #[cfg(not(windows))]
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Acquire) {
        Err(LibraryError::new(
            "CANCELLED",
            "Operation cancelled. No incomplete artifact was committed.",
        ))
    } else {
        Ok(())
    }
}
fn reparse(m: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0 || m.file_type().is_symlink()
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}
pub fn safe_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(LibraryError::new(
            "UNSAFE_PATH",
            "Select an absolute local path without traversal.",
        ));
    }
    #[cfg(windows)]
    {
        use std::path::Prefix;
        if !matches!(path.components().next(),Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_)))
            || path
                .components()
                .any(|c| matches!(c,Component::Normal(v) if v.to_string_lossy().contains(':')))
        {
            return Err(LibraryError::new(
                "UNSAFE_PATH",
                "Network/device paths and alternate data streams are not supported.",
            ));
        }
        // A mapped network share also has a drive-letter prefix. Reject it before I/O.
        if let Some(Component::Prefix(prefix)) = path.components().next() {
            let drive = match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => unreachable!(),
            };
            let root = [u16::from(drive), b':' as u16, b'\\' as u16, 0];
            const DRIVE_REMOTE: u32 = 4; // GetDriveTypeW return value (WinBase.h).
            if unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root.as_ptr()) }
                == DRIVE_REMOTE
            {
                return Err(LibraryError::new(
                    "UNSAFE_PATH",
                    "Mapped network drives are not supported. Select a local file or folder.",
                ));
            }
        }
    }
    Ok(())
}
pub fn guard_directories(path: &Path) -> Result<Vec<File>> {
    safe_path(path)?;
    let mut guards = vec![];
    let mut part = PathBuf::new();
    for c in path.components() {
        part.push(c);
        if matches!(c, Component::Prefix(_) | Component::RootDir) {
            continue;
        }
        let m = fs::symlink_metadata(&part)?;
        if !m.is_dir() || reparse(&m) {
            return Err(LibraryError::new(
                "UNSAFE_PATH",
                "Linked/reparse-point directories are not supported. Select a normal local folder.",
            ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            guards.push(
                OpenOptions::new()
                    .access_mode(0x80000000) // GENERIC_READ activates share-delete protection.
                    .share_mode(3) // Allow child writes, deny deletion/rename of the leased directory.
                    .custom_flags(0x02000000 | 0x00200000)
                    .open(&part)
                    .map_err(|e| {
                        #[cfg(test)]
                        eprintln!("Directory lease failed at {part:?}: {e}");
                        LibraryError::from(e)
                    })?,
            );
        }
    }
    Ok(guards)
}
pub fn directory(path: &Path) -> Result<Vec<File>> {
    if !path.exists() {
        let parent = path
            .parent()
            .ok_or_else(|| LibraryError::new("UNSAFE_PATH", "Storage has no parent."))?;
        let _g = guard_directories(parent)?;
        fs::create_dir(path)?;
    }
    guard_directories(path)
}
pub fn read_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1).custom_flags(0x00200000 | 0x08000000);
    }
    let f = options.open(path)?;
    let m = f.metadata()?;
    if !m.is_file() || reparse(&m) {
        return Err(LibraryError::new(
            "INVALID_SOURCE",
            "Only regular local files without reparse points are supported.",
        ));
    }
    Ok(f)
}
pub fn new_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1).custom_flags(0x00200000);
    }
    Ok(options.open(path)?)
}
pub fn filename(path: &Path) -> Result<String> {
    let value = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| LibraryError::new("INVALID_NAME", "Filename is not valid Unicode."))?;
    if value.is_empty()
        || value.len() > 1024
        || value.chars().any(char::is_control)
        || value.contains(['/', '\\'])
    {
        return Err(LibraryError::new(
            "INVALID_NAME",
            "Filename is empty, too long or contains invalid characters.",
        ));
    }
    Ok(value.into())
}
pub fn inside(path: &Path, root: &Path) -> Result<bool> {
    let path = if path.exists() {
        fs::canonicalize(path)?
    } else {
        fs::canonicalize(
            path.parent()
                .ok_or_else(|| LibraryError::new("UNSAFE_PATH", "Path has no parent."))?,
        )?
        .join(
            path.file_name()
                .ok_or_else(|| LibraryError::new("INVALID_NAME", "Path has no filename."))?,
        )
    };
    let root = fs::canonicalize(root)?;
    #[cfg(windows)]
    {
        Ok(PathBuf::from(path.to_string_lossy().to_lowercase())
            .starts_with(PathBuf::from(root.to_string_lossy().to_lowercase())))
    }
    #[cfg(not(windows))]
    {
        Ok(path.starts_with(root))
    }
}
pub fn unchanged(before: &Metadata, after: &Metadata) -> bool {
    before.len() == after.len() && before.modified().ok() == after.modified().ok()
}
pub fn stream<R: Read, W: Write>(
    source: &mut R,
    target: &mut W,
    limit: u64,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64) -> Result<()>,
) -> Result<(String, u64)> {
    let mut buffer = vec![0; CHUNK];
    let mut digest = Sha256::new();
    let mut bytes = 0;
    loop {
        cancelled(cancel)?;
        let n = source.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > limit {
            return Err(LibraryError::new(
                "SOURCE_CHANGED",
                "Source grew beyond its approved size; select it again.",
            ));
        }
        target.write_all(&buffer[..n])?;
        digest.update(&buffer[..n]);
        progress(bytes)?;
    }
    cancelled(cancel)?;
    Ok((format!("{:x}", digest.finalize()), bytes))
}
pub fn hash_file(
    path: &Path,
    limit: u64,
    cancel: &AtomicBool,
    progress: impl FnMut(u64) -> Result<()>,
) -> Result<(File, String, u64)> {
    let mut f = read_file(path)?;
    let (hash, size) = stream(&mut f, &mut std::io::sink(), limit, cancel, progress)?;
    Ok((f, hash, size))
}
pub fn free_space(path: &Path, needed: u64) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut free = 0;
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(LibraryError::new(
                "SPACE_UNAVAILABLE",
                "Available storage could not be measured. Check the local vault volume.",
            ));
        }
        if free < needed.saturating_add(16 * 1024 * 1024) {
            return Err(LibraryError::new(
                "DISK_FULL",
                "Not enough space for staging plus the storage reserve.",
            ));
        }
    }
    Ok(())
}
pub struct Vault {
    pub root: PathBuf,
    pub staging: PathBuf,
    _guards: Vec<File>,
    _lock: Option<File>,
    pub unavailable: bool,
}
impl Vault {
    pub fn open(root: &Path) -> Result<Self> {
        let mut guards = directory(root)?;
        let staging = root.join("staging");
        guards.extend(directory(&staging)?);
        guards.extend(directory(&root.join("sha256"))?);
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0).custom_flags(0x00200000);
        }
        let _lock=options.open(root.join("library.lock")).map_err(|_|LibraryError::new("VAULT_BUSY","Another OBTAINER process owns the vault, or storage is unavailable. Close the other instance and retry."))?;
        Ok(Self {
            root: root.into(),
            staging,
            _guards: guards,
            _lock: Some(_lock),
            unavailable: false,
        })
    }
    /// Preserve a configured but unavailable location without creating a replacement.
    pub fn unavailable(root: PathBuf) -> Self {
        Self {
            staging: root.join("staging"),
            root,
            _guards: vec![],
            _lock: None,
            unavailable: true,
        }
    }
    pub fn ensure_available(&self) -> Result<()> {
        if self.unavailable {
            return Err(LibraryError::new("LIBRARY_UNAVAILABLE", "Library storage is unavailable. Reconnect its drive or restore the configured folder, then restart OBTAINER. Existing records and settings are retained."));
        }
        Ok(())
    }
    pub fn stage(&self, id: &str) -> Result<PathBuf> {
        self.ensure_available()?;
        if !valid_id(id) {
            return Err(LibraryError::new(
                "INVALID_JOB",
                "Invalid staging identity.",
            ));
        }
        Ok(self.staging.join(format!("{id}.part")))
    }
    pub fn blob(&self, hash: &str) -> Result<(PathBuf, Vec<File>)> {
        self.ensure_available()?;
        if !valid_hash(hash) {
            return Err(LibraryError::new(
                "INVALID_HASH",
                "Invalid artifact identity.",
            ));
        }
        let dir = self.root.join("sha256").join(&hash[..2]);
        let guards = directory(&dir)?;
        Ok((dir.join(format!("{hash}.bin")), guards))
    }
    pub fn key(hash: &str) -> String {
        format!("sha256/{}/{}.bin", &hash[..2], hash)
    }
    pub fn remove_stage(&self, id: &str) -> Result<()> {
        let p = self.stage(id)?;
        match fs::symlink_metadata(&p) {
            Ok(m) if m.is_file() && !reparse(&m) => {
                fs::remove_file(p)?;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            _ => Err(LibraryError::new(
                "STAGING_RETAINED",
                "Unexpected staging item retained for manual review.",
            )),
        }
    }
}

#[cfg(windows)]
pub fn dialog(save: bool, owner: isize) -> Result<Option<PathBuf>> {
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let mut buffer = vec![0u16; 32768];
    let title: Vec<u16> = if save {
        "Export artifact manifest"
    } else {
        "Import firmware"
    }
    .encode_utf16()
    .chain(Some(0))
    .collect();
    let filter: Vec<u16> = if save {
        "JSON manifest\0*.json\0\0"
    } else {
        "Binary files\0*.bin;*.img;*.fw\0All files\0*.*\0\0"
    }
    .encode_utf16()
    .collect();
    let extension: Vec<u16> = "json\0".encode_utf16().collect();
    let mut info: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    info.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    info.hwndOwner = owner as _;
    info.lpstrFile = buffer.as_mut_ptr();
    info.nMaxFile = buffer.len() as u32;
    info.lpstrTitle = title.as_ptr();
    info.lpstrFilter = filter.as_ptr();
    info.nFilterIndex = 1;
    info.Flags = OFN_EXPLORER
        | OFN_PATHMUSTEXIST
        | OFN_NOCHANGEDIR
        | OFN_DONTADDTORECENT
        | if save {
            OFN_NOREADONLYRETURN
        } else {
            OFN_FILEMUSTEXIST
        };
    if save {
        info.lpstrDefExt = extension.as_ptr();
    }
    let ok = unsafe {
        if save {
            GetSaveFileNameW(&mut info)
        } else {
            GetOpenFileNameW(&mut info)
        }
    };
    if ok == 0 {
        if unsafe { CommDlgExtendedError() } == 0 {
            return Ok(None);
        }
        return Err(LibraryError::new(
            "DIALOG_FAILED",
            "Windows file dialog failed. Retry from the desktop app.",
        ));
    }
    let end = buffer.iter().position(|v| *v == 0).unwrap_or(buffer.len());
    Ok(Some(PathBuf::from(
        String::from_utf16(&buffer[..end]).map_err(|_| {
            LibraryError::new("INVALID_NAME", "Selected path is not valid Unicode.")
        })?,
    )))
}
#[cfg(not(windows))]
pub fn dialog(_: bool, _: isize) -> Result<Option<PathBuf>> {
    Err(LibraryError::new(
        "WINDOWS_REQUIRED",
        "Native file selection requires Windows.",
    ))
}
