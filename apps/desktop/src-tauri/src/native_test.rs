//! Test-only storage is derived from the Windows known folder, never an input path.
use std::io;

pub fn reject_overrides(test_enabled: bool) -> io::Result<Option<String>> {
    validate_overrides(std::env::vars_os(), test_enabled)
}
fn validate_overrides(
    vars: impl IntoIterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
    test_enabled: bool,
) -> io::Result<Option<String>> {
    let mut run = None;
    for (key, value) in vars {
        let key = key.to_string_lossy().to_ascii_uppercase();
        if key == "OBTAINER_NATIVE_TEST_RUN_ID" && test_enabled {
            let value = value.to_str().ok_or_else(invalid)?;
            if !crate::library::types::valid_id(value) {
                return Err(invalid());
            }
            run = Some(value.to_owned());
        } else if key.starts_with("OBTAINER_NATIVE_TEST_")
            || matches!(key.as_str(), "OBTAINER_DATA_DIR" | "OBTAINER_TEST_DATA_DIR")
        {
            return Err(invalid());
        }
    }
    if test_enabled && run.is_none() {
        return Err(invalid());
    }
    Ok(run)
}
fn invalid() -> io::Error {
    io::Error::other("Native tests require the dedicated test executable and a 32-character lowercase hexadecimal run ID. Data-path overrides are forbidden.")
}

#[cfg(any(feature = "native-test-profile", test))]
mod isolated {
    use super::*;
    use crate::library::files::guard_directories;
    use serde::{Deserialize, Serialize};
    use std::{
        fs::{self, File, OpenOptions},
        path::{Path, PathBuf},
    };
    const MARKER: &str = ".obtainer-native-test.json";
    const OWNER: &str = ".native-owner.lock";
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Marker {
        version: u8,
        kind: String,
        run_id: String,
        state: String,
    }
    pub struct TestRun {
        pub root: PathBuf,
        pub id: String,
        _owner: File,
        _leases: Vec<File>,
    }
    fn error(message: impl ToString) -> io::Error {
        io::Error::other(message.to_string())
    }
    fn validate_id(id: &str) -> io::Result<()> {
        if crate::library::valid_id(id) {
            Ok(())
        } else {
            Err(invalid())
        }
    }
    fn ordinary(path: &Path) -> io::Result<fs::Metadata> {
        let m = fs::symlink_metadata(path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if m.file_attributes() & 0x400 != 0 {
                return Err(error("Reparse point retained; cleanup refused."));
            }
        }
        if m.file_type().is_symlink() {
            return Err(error("Symbolic link retained; cleanup refused."));
        }
        Ok(m)
    }
    fn mkdir(parent: &Path, name: &str) -> io::Result<PathBuf> {
        let p = parent.join(name);
        match fs::create_dir(&p) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        if !ordinary(&p)?.is_dir() {
            return Err(error("Test location is not a normal directory."));
        }
        Ok(p)
    }
    fn owner(path: &Path) -> io::Result<File> {
        if path.exists() && !ordinary(path)?.is_file() {
            return Err(error("Invalid ownership file."));
        }
        let mut opts = OpenOptions::new();
        opts.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.share_mode(0).custom_flags(0x00200000);
        }
        let f = opts.open(path)?;
        let metadata = f.metadata()?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(error("Linked ownership handle refused."));
            }
        }
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(error("Invalid ownership handle."));
        }
        Ok(f)
    }
    fn marker(root: &Path, id: &str, cleaning: bool) -> io::Result<Marker> {
        let p = root.join(MARKER);
        if !ordinary(&p)?.is_file() || ordinary(&p)?.len() > 4096 {
            return Err(error("Missing test marker."));
        }
        let m: Marker = serde_json::from_slice(&fs::read(p)?).map_err(error)?;
        if m.version != 1
            || m.kind != "OBTAINER_NATIVE_TEST"
            || m.run_id != id
            || !(m.state == "active" || (cleaning && m.state == "cleaning"))
        {
            return Err(error("Test marker/run ID does not match; retained."));
        }
        Ok(m)
    }
    impl TestRun {
        pub fn open(local: &Path, id: &str) -> io::Result<Self> {
            validate_id(id)?;
            let mut leases = guard_directories(local).map_err(error)?;
            let base = mkdir(local, "OBTAINER-NativeTests")?;
            leases.extend(guard_directories(&base).map_err(error)?);
            let root = base.join(id);
            let fresh = match fs::create_dir(&root) {
                Ok(()) => true,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => false,
                Err(e) => return Err(e),
            };
            leases.extend(guard_directories(&root).map_err(error)?);
            let owner = owner(&root.join(OWNER))?;
            if fresh {
                let m = Marker {
                    version: 1,
                    kind: "OBTAINER_NATIVE_TEST".into(),
                    run_id: id.into(),
                    state: "active".into(),
                };
                let mut f = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(root.join(MARKER))?;
                use std::io::Write;
                f.write_all(&serde_json::to_vec(&m).map_err(error)?)?;
                f.sync_all()?;
            } else {
                marker(&root, id, false)?;
            }
            for name in ["data", "webview", "fixtures", "exports"] {
                let directory = mkdir(&root, name)?;
                leases.extend(guard_directories(&directory).map_err(error)?);
            }
            for name in ["logs", "vault", "workspaces", "reports"] {
                let directory = mkdir(&root.join("data"), name)?;
                leases.extend(guard_directories(&directory).map_err(error)?);
            }
            Ok(Self {
                root,
                id: id.into(),
                _owner: owner,
                _leases: leases,
            })
        }
    }
    fn inspect_tree(path: &Path) -> io::Result<()> {
        let m = ordinary(path)?;
        if m.is_dir() {
            let _lease = guard_directories(path).map_err(error)?;
            if path.ends_with("vault/staging") && fs::read_dir(path)?.next().is_some() {
                return Err(error(
                    "Nonempty staging retained for review; cleanup refused.",
                ));
            }
            for item in fs::read_dir(path)? {
                inspect_tree(&item?.path())?;
            }
        } else if !m.is_file() {
            return Err(error("Unknown filesystem item retained."));
        }
        Ok(())
    }
    // Each directory is leased while traversed. Removing a file/link never follows it.
    fn remove_tree(path: &Path) -> io::Result<()> {
        let leases = guard_directories(path).map_err(error)?;
        for item in fs::read_dir(path)? {
            let p = item?.path();
            let m = ordinary(&p)?;
            if m.is_dir() {
                remove_tree(&p)?;
            } else {
                fs::remove_file(p)?;
            }
        }
        drop(leases);
        fs::remove_dir(path)
    }
    pub fn cleanup(local: &Path, id: &str) -> io::Result<()> {
        validate_id(id)?;
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let status=std::process::Command::new("powershell.exe")
                .args(["-NoProfile","-Command","$ErrorActionPreference='Stop'; try { $active=@(Get-CimInstance Win32_Process | Where-Object { ($_.Name -eq 'obtainer-native-test.exe' -and $_.ProcessId -ne [uint32]$env:OBTAINER_CLEANUP_SELF_PID) -or ($_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -like ('*'+$env:OBTAINER_CLEANUP_RUN_ID+'*')) }); if($active.Count -gt 0){exit 2}; exit 0 } catch {exit 3}"])
                .env("OBTAINER_CLEANUP_SELF_PID",std::process::id().to_string())
                .env("OBTAINER_CLEANUP_RUN_ID",id)
                .creation_flags(0x08000000)
                .status()?;
            if !status.success() {
                return Err(error("Native-test/WebView process still exists, or process ownership could not be checked; retained."));
            }
        }
        let base = local.join("OBTAINER-NativeTests");
        let root = base.join(id);
        let _parents = guard_directories(&base).map_err(error)?;
        let leases = guard_directories(&root).map_err(error)?;
        if fs::canonicalize(&root)?.parent() != Some(fs::canonicalize(&base)?.as_path()) {
            return Err(error("Test root boundary mismatch."));
        }
        let owner = owner(&root.join(OWNER)).map_err(|_| {
            error("A process owns this test run, or its ownership cannot be verified; retained.")
        })?;
        let mut m = marker(&root, id, true)?;
        for item in fs::read_dir(&root)? {
            let p = item?.path();
            let name = p.file_name().and_then(|s| s.to_str()).ok_or_else(invalid)?;
            if ![MARKER, OWNER, "data", "webview", "fixtures", "exports"].contains(&name) {
                return Err(error("Unknown run-root item retained."));
            }
            inspect_tree(&p)?;
        }
        // Once cleanup starts, another startup cannot adopt this directory after lock release.
        m.state = "cleaning".into();
        fs::write(root.join(MARKER), serde_json::to_vec(&m).map_err(error)?)?;
        for name in ["data", "webview", "fixtures", "exports"] {
            let p = root.join(name);
            if p.exists() {
                remove_tree(&p)?;
            }
        }
        drop(owner);
        fs::remove_file(root.join(OWNER))?;
        fs::remove_file(root.join(MARKER))?;
        drop(leases);
        fs::remove_dir(root)
    }
    #[cfg(all(windows, feature = "native-test-profile"))]
    pub fn local_data() -> io::Result<PathBuf> {
        use windows_sys::Win32::{
            System::Com::CoTaskMemFree,
            UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
        };
        let mut raw = std::ptr::null_mut();
        if unsafe {
            SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, std::ptr::null_mut(), &mut raw)
        } < 0
        {
            return Err(error("Windows LocalAppData known folder unavailable."));
        }
        let mut n = 0;
        unsafe {
            while *raw.add(n) != 0 {
                n += 1;
            }
            let text = String::from_utf16(std::slice::from_raw_parts(raw, n)).map_err(error);
            CoTaskMemFree(raw.cast());
            text.map(PathBuf::from)
        }
    }
    #[cfg(all(not(windows), feature = "native-test-profile"))]
    pub fn local_data() -> io::Result<PathBuf> {
        Err(error("Windows native tests required."))
    }

    #[cfg(all(test, windows))]
    mod tests {
        use super::*;
        const ID: &str = "0123456789abcdef0123456789abcdef";
        #[test]
        fn run_id_is_not_a_path() {
            for id in [
                "",
                "../OBTAINER",
                "C:\\OBTAINER",
                "A123456789abcdef0123456789abcdef",
            ] {
                assert!(validate_id(id).is_err());
            }
        }
        #[test]
        fn isolation_restart_and_cleanup_preserve_unrelated_files() {
            let t = tempfile::tempdir().unwrap();
            let real = t.path().join("OBTAINER");
            fs::create_dir(&real).unwrap();
            fs::write(real.join("keep"), b"user data").unwrap();
            let r = TestRun::open(t.path(), ID).unwrap();
            assert!(cleanup(t.path(), ID).is_err());
            assert_eq!(r.id, ID);
            let root = r.root.clone();
            drop(r);
            let r = TestRun::open(t.path(), ID).unwrap();
            assert_eq!(r.root, root);
            drop(r);
            cleanup(t.path(), ID).unwrap();
            assert!(!root.exists());
            assert_eq!(fs::read(real.join("keep")).unwrap(), b"user data");
        }
        #[test]
        fn unknown_marker_staging_and_root_entries_are_retained() {
            let t = tempfile::tempdir().unwrap();
            let r = TestRun::open(t.path(), ID).unwrap();
            let root = r.root.clone();
            drop(r);
            fs::write(root.join("unknown"), b"keep").unwrap();
            assert!(cleanup(t.path(), ID).is_err());
            fs::remove_file(root.join("unknown")).unwrap();
            let stage = root.join("data/vault/staging");
            fs::create_dir_all(&stage).unwrap();
            fs::write(stage.join("unknown.part"), b"keep").unwrap();
            assert!(cleanup(t.path(), ID).is_err());
            fs::remove_file(stage.join("unknown.part")).unwrap();
            fs::write(root.join(MARKER), b"{}").unwrap();
            assert!(cleanup(t.path(), ID).is_err());
            assert!(root.exists());
        }
        #[cfg(windows)]
        #[test]
        fn cleanup_rejects_junction_without_touching_target() {
            let t = tempfile::tempdir().unwrap();
            let target = tempfile::tempdir().unwrap();
            fs::write(target.path().join("keep"), b"unrelated").unwrap();
            let r = TestRun::open(t.path(), ID).unwrap();
            let root = r.root.clone();
            drop(r);
            let link = root.join("fixtures/link");
            let status=std::process::Command::new("powershell.exe").args(["-NoProfile","-Command","$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:OBTAINER_TEST_LINK -Target $env:OBTAINER_TEST_TARGET | Out-Null"]).env("OBTAINER_TEST_LINK",&link).env("OBTAINER_TEST_TARGET",target.path()).status().unwrap();
            assert!(status.success());
            assert!(cleanup(t.path(), ID).is_err());
            assert_eq!(fs::read(target.path().join("keep")).unwrap(), b"unrelated");
            fs::remove_dir(link).unwrap();
            cleanup(t.path(), ID).unwrap();
        }
    }
}
#[cfg(feature = "native-test-profile")]
pub use isolated::{cleanup, local_data, TestRun};

#[cfg(test)]
mod activation_tests {
    use super::*;
    #[test]
    fn production_cannot_activate_test_mode_and_paths_are_rejected() {
        let id = "0123456789abcdef0123456789abcdef";
        let vars =
            |k: &str, v: &str| vec![(std::ffi::OsString::from(k), std::ffi::OsString::from(v))];
        assert!(validate_overrides(vars("OBTAINER_NATIVE_TEST_RUN_ID", id), false).is_err());
        assert!(
            validate_overrides(vars("OBTAINER_NATIVE_TEST_RUN_ID", "../OBTAINER"), true).is_err()
        );
        for key in [
            "OBTAINER_DATA_DIR",
            "OBTAINER_TEST_DATA_DIR",
            "OBTAINER_NATIVE_TEST_PATH",
        ] {
            assert!(validate_overrides(vars(key, "C:/user-data"), true).is_err());
        }
        assert!(validate_overrides(Vec::new(), true).is_err());
        assert_eq!(
            validate_overrides(vars("OBTAINER_NATIVE_TEST_RUN_ID", id), true)
                .unwrap()
                .as_deref(),
            Some(id)
        );
        assert_eq!(validate_overrides(Vec::new(), false).unwrap(), None);
    }
}
