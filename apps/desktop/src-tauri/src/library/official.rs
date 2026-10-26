//! Closed, release-pinned public firmware sources. No device endpoint or credential is accepted.
use super::{files::VerifiedPublishedBlob, types::*};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs::File, sync::atomic::AtomicBool};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfficialSource {
    pub id: String,
    pub vendor: String,
    pub model: String,
    pub revision: String,
    pub profile: String,
    pub target: String,
    pub version: String,
    pub filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub url: String,
    pub manifest_url: String,
    pub manifest_sha256: String,
    pub profiles_sha256: String,
    pub signing_fingerprint: String,
    pub primary_fingerprint: String,
    pub verification: String,
    pub installability: String,
}
pub(super) struct QualifiedSource(OfficialSource);
impl QualifiedSource {
    pub(super) fn entry(&self) -> &OfficialSource {
        &self.0
    }
    pub(super) fn verify(&self, proof: &VerifiedPublishedBlob) -> Result<()> {
        verify_identity(&self.0, proof.hash(), proof.size())
    }
}
pub fn catalogue() -> Result<Vec<OfficialSource>> {
    let sources: Vec<OfficialSource> =
        serde_json::from_str(include_str!("../../assets/official/catalogue.json"))
            .map_err(|_| invalid())?;
    let manifest = include_bytes!("../../assets/official/sha256sums");
    let manifest_hash = format!("{:x}", Sha256::digest(manifest));
    let manifest = std::str::from_utf8(manifest).map_err(|_| invalid())?;
    let mut ids = std::collections::HashSet::new();
    for source in &sources {
        let prefix = format!(
            "https://downloads.openwrt.org/releases/{}/targets/{}/",
            source.version, source.target
        );
        let url = url::Url::parse(&source.url).map_err(|_| invalid())?;
        if !ids.insert(&source.id)
            || source.size_bytes == 0
            || source.size_bytes > 128 * 1024 * 1024
            || !valid_hash(&source.sha256)
            || source.manifest_sha256 != manifest_hash
            || source.filename.contains(['/', '\\', ':', '%', '?', '#'])
            || source.filename.contains("..")
            || !source.filename.ends_with("-sysupgrade.bin")
            || source.url != format!("{prefix}{}", source.filename)
            || source.manifest_url != format!("{prefix}sha256sums")
            || url.scheme() != "https"
            || url.host_str() != Some("downloads.openwrt.org")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || source.signing_fingerprint != "92C561DE55AE6552F3C736B82B0151090606D1D9"
            || source.primary_fingerprint != "8A8BC12F46B836C0F9CDB36F1D53D1877742E911"
            || source.verification != "pinned_catalogue_from_verified_release_signature"
            || source.installability != "unknown"
            || manifest
                .lines()
                .filter(|line| *line == format!("{} *{}", source.sha256, source.filename))
                .count()
                != 1
        {
            return Err(invalid());
        }
    }
    Ok(sources)
}
pub(super) fn qualify(id: &str, confirmed: bool) -> Result<QualifiedSource> {
    if !confirmed {
        return Err(LibraryError::new("SOURCE_APPROVAL_REQUIRED", "Confirm the exact catalogue model and revision before downloading. This does not verify connected hardware."));
    }
    catalogue()?
        .into_iter()
        .find(|s| s.id == id)
        .map(QualifiedSource)
        .ok_or_else(|| {
            LibraryError::new(
                "SOURCE_NOT_APPROVED",
                "No approved exact-match firmware source. No request was sent.",
            )
        })
}
fn invalid() -> LibraryError {
    LibraryError::new(
        "INVALID_CATALOGUE",
        "The compiled firmware catalogue failed validation. No source was authorized.",
    )
}
fn verify_identity(source: &OfficialSource, hash: &str, size: u64) -> Result<()> {
    if hash != source.sha256 || size != source.size_bytes {
        return Err(LibraryError::new("OFFICIAL_HASH_MISMATCH", "Downloaded bytes do not match the exact approved release image. No firmware was admitted."));
    }
    Ok(())
}
impl super::LibraryService {
    pub fn official_provenance(&self, hash: &str) -> Result<Vec<OfficialSource>> {
        self.details(hash)?;
        let all = catalogue()?;
        let repo = super::lock(&self.inner.repo)?;
        let mut statement=repo.connection.prepare("SELECT source_reference FROM artifact_occurrences WHERE blob_sha256=?1 AND artifact_class='official_image_download' AND provenance_kind='official_vendor_download' ORDER BY collected_at DESC LIMIT 50")?;
        let rows = statement
            .query_map([hash], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(|raw| {
                let source: OfficialSource = serde_json::from_str(&raw).map_err(|_| invalid())?;
                if source.sha256 != hash || !all.contains(&source) {
                    return Err(invalid());
                }
                Ok(source)
            })
            .collect()
    }
}
pub(super) fn download(
    source: &QualifiedSource,
    output: &mut File,
    cancel: &AtomicBool,
    progress: impl FnMut(u64) -> Result<()>,
) -> Result<()> {
    #[cfg(windows)]
    {
        windows::download(source, output, cancel, progress)
    }
    #[cfg(not(windows))]
    {
        let _ = (source, output, cancel, progress);
        Err(LibraryError::new(
            "WINDOWS_REQUIRED",
            "Official downloads require the Windows desktop transport.",
        ))
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        ffi::c_void,
        io::Write,
        ptr,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::Networking::WinHttp::*;
    struct Handle(*mut c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
    fn handle(value: *mut c_void) -> Result<Handle> {
        if value.is_null() {
            Err(network())
        } else {
            Ok(Handle(value))
        }
    }
    fn network() -> LibraryError {
        LibraryError::new("DOWNLOAD_FAILED","Official HTTPS request failed or timed out. Retry from the catalogue; no incomplete firmware was admitted.")
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn set(handle: &Handle, option: u32, value: u32) -> Result<()> {
        if unsafe { WinHttpSetOption(handle.0, option, (&value as *const u32).cast(), 4) } == 0 {
            Err(network())
        } else {
            Ok(())
        }
    }
    fn check(cancel: &AtomicBool, deadline: Instant) -> Result<()> {
        super::super::files::cancelled(cancel)?;
        if Instant::now() >= deadline {
            return Err(LibraryError::new(
                "DOWNLOAD_TIMEOUT",
                "Download exceeded its three-minute budget. No incomplete image was admitted.",
            ));
        }
        Ok(())
    }
    pub(super) fn download(
        source: &QualifiedSource,
        output: &mut File,
        cancel: &AtomicBool,
        progress: impl FnMut(u64) -> Result<()>,
    ) -> Result<()> {
        request(
            source,
            output,
            cancel,
            progress,
            ("downloads.openwrt.org", 443, WINHTTP_FLAG_SECURE),
        )
    }
    fn request(
        source: &QualifiedSource,
        output: &mut File,
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64) -> Result<()>,
        endpoint: (&str, u16, u32),
    ) -> Result<()> {
        let source = source.entry();
        let deadline = Instant::now() + Duration::from_secs(180);
        check(cancel, deadline)?;
        let url = url::Url::parse(&source.url).map_err(|_| invalid())?;
        let session = handle(unsafe {
            WinHttpOpen(
                wide("OBTAINER official-image beta").as_ptr(),
                WINHTTP_ACCESS_TYPE_NO_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            )
        })?;
        if unsafe { WinHttpSetTimeouts(session.0, 5000, 10000, 5000, 5000) } == 0 {
            return Err(network());
        }
        let connection =
            handle(unsafe { WinHttpConnect(session.0, wide(endpoint.0).as_ptr(), endpoint.1, 0) })?;
        let request = handle(unsafe {
            WinHttpOpenRequest(
                connection.0,
                wide("GET").as_ptr(),
                wide(url.path()).as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                endpoint.2,
            )
        })?;
        set(
            &request,
            WINHTTP_OPTION_DISABLE_FEATURE,
            WINHTTP_DISABLE_REDIRECTS
                | WINHTTP_DISABLE_AUTHENTICATION
                | WINHTTP_DISABLE_COOKIES
                | WINHTTP_DISABLE_KEEP_ALIVE,
        )?;
        set(
            &request,
            WINHTTP_OPTION_AUTOLOGON_POLICY,
            WINHTTP_AUTOLOGON_SECURITY_LEVEL_HIGH,
        )?;
        set(&request, WINHTTP_OPTION_MAX_RESPONSE_HEADER_SIZE, 8192)?;
        check(cancel, deadline)?;
        if unsafe { WinHttpSendRequest(request.0, ptr::null(), 0, ptr::null(), 0, 0, 0) } == 0 {
            check(cancel, deadline)?;
            return Err(network());
        }
        check(cancel, deadline)?;
        if unsafe { WinHttpReceiveResponse(request.0, ptr::null_mut()) } == 0 {
            check(cancel, deadline)?;
            return Err(network());
        }
        let mut status = 0u32;
        let mut size = 4;
        if unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                ptr::null(),
                (&mut status as *mut u32).cast(),
                &mut size,
                ptr::null_mut(),
            )
        } == 0
            || status != 200
        {
            return Err(LibraryError::new("DOWNLOAD_HTTP_REJECTED","Official source did not return HTTP 200. Redirects and authentication are never followed."));
        }
        let mut total = 0u64;
        let mut bytes = [0u8; 65536];
        loop {
            check(cancel, deadline)?;
            let mut count = 0;
            if unsafe {
                WinHttpReadData(
                    request.0,
                    bytes.as_mut_ptr().cast(),
                    bytes.len() as u32,
                    &mut count,
                )
            } == 0
            {
                check(cancel, deadline)?;
                return Err(network());
            }
            check(cancel, deadline)?;
            if count == 0 {
                break;
            }
            total += u64::from(count);
            if total > source.size_bytes {
                return Err(LibraryError::new(
                    "DOWNLOAD_TOO_LARGE",
                    "Response exceeded the exact approved image size. No firmware admitted.",
                ));
            }
            output.write_all(&bytes[..count as usize])?;
            progress(total)?;
        }
        if total != source.size_bytes {
            return Err(LibraryError::new(
                "DOWNLOAD_TRUNCATED",
                "Official image response ended before its approved size. No firmware admitted.",
            ));
        }
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{io::Read, net::TcpListener, sync::atomic::Ordering, thread};
        // Synthetic HTTP loopback bytes. Production can call only the fixed HTTPS wrapper.
        fn synthetic_response(
            status: &str,
            body: Vec<u8>,
            expected: u64,
            cancel_after_chunk: bool,
        ) -> Result<()> {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            listener.set_nonblocking(true).unwrap();
            let response=format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:{port}/forbidden\r\nConnection: close\r\n\r\n",body.len());
            let server = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut socket = loop {
                    match listener.accept() {
                        Ok((s, _)) => break s,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "Synthetic peer never connected");
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(e) => panic!("{e}"),
                    }
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut headers = Vec::new();
                while !headers.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).unwrap();
                    headers.push(byte[0]);
                    assert!(headers.len() < 8192);
                }
                let text = String::from_utf8(headers).unwrap().to_lowercase();
                assert!(text.starts_with("get /synthetic "));
                assert!(!text.contains("authorization:"));
                assert!(!text.contains("cookie:"));
                socket.write_all(response.as_bytes()).unwrap();
                let _ = socket.write_all(&body);
            });
            let mut entry = catalogue().unwrap().remove(0);
            entry.url = format!("http://127.0.0.1:{port}/synthetic");
            entry.size_bytes = expected;
            let source = QualifiedSource(entry);
            let cancel = AtomicBool::new(false);
            let mut file = tempfile::tempfile().unwrap();
            let result = request(
                &source,
                &mut file,
                &cancel,
                |_| {
                    if cancel_after_chunk {
                        cancel.store(true, Ordering::Release);
                    }
                    Ok(())
                },
                ("127.0.0.1", port, 0),
            );
            server.join().unwrap();
            result
        }
        #[test]
        fn synthetic_transport_rejects_redirects_authentication_truncation_and_oversize() {
            for status in ["302 Found", "401 Unauthorized", "403 Forbidden"] {
                assert_eq!(
                    synthetic_response(status, b"fixture".to_vec(), 7, false)
                        .unwrap_err()
                        .code,
                    "DOWNLOAD_HTTP_REJECTED"
                );
            }
            assert_eq!(
                synthetic_response("200 OK", b"short".to_vec(), 10, false)
                    .unwrap_err()
                    .code,
                "DOWNLOAD_TRUNCATED"
            );
            assert_eq!(
                synthetic_response("200 OK", b"oversized".to_vec(), 4, false)
                    .unwrap_err()
                    .code,
                "DOWNLOAD_TOO_LARGE"
            );
            synthetic_response("200 OK", b"synthetic".to_vec(), 9, false).unwrap();
        }
        #[test]
        fn cancellation_and_expiry_release_transport_and_next_request_works() {
            assert_eq!(
                synthetic_response("200 OK", vec![0; 131072], 131072, true)
                    .unwrap_err()
                    .code,
                "CANCELLED"
            );
            assert_eq!(
                check(
                    &AtomicBool::new(false),
                    Instant::now() - Duration::from_secs(1)
                )
                .unwrap_err()
                .code,
                "DOWNLOAD_TIMEOUT"
            );
            synthetic_response("200 OK", b"ok".to_vec(), 2, false).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_is_signed_manifest_pinned_exact_target_data_not_device_authority() {
        let all = catalogue().unwrap();
        assert_eq!(all.len(), 3);
        for item in all {
            assert_eq!(item.installability, "unknown");
            assert!(qualify(&item.id, true).is_ok());
        }
    }
    #[test]
    fn arbitrary_urls_labels_wrong_revisions_and_unconfirmed_sources_cannot_authorize_io() {
        for id in [
            "https://evil.invalid/image.bin",
            "http://127.0.0.1/",
            "Archer C7",
            "../source",
            "openwrt-25.12.6-tplink_archer-c7-v4",
            "",
        ] {
            assert!(qualify(id, true).is_err());
        }
        assert!(qualify(&catalogue().unwrap()[0].id, false).is_err());
    }
    #[test]
    fn exact_size_and_hash_both_required_and_cancel_prevents_network() {
        let item = qualify(&catalogue().unwrap()[0].id, true).unwrap();
        let e = item.entry();
        assert!(verify_identity(e, &e.sha256, e.size_bytes).is_ok());
        assert!(verify_identity(e, &"0".repeat(64), e.size_bytes).is_err());
        assert!(verify_identity(e, &e.sha256, e.size_bytes - 1).is_err());
        let mut output = tempfile::tempfile().unwrap();
        assert_eq!(
            download(&item, &mut output, &AtomicBool::new(true), |_| Ok(()))
                .unwrap_err()
                .code,
            "CANCELLED"
        );
        assert_eq!(output.metadata().unwrap().len(), 0);
    }
}
