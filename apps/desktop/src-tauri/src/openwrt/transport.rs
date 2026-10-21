//! Bounded Windows OpenSSH invocation. No shell, user config, password helper or agent.
use super::query::ReadQuery;
use crate::adapters::{AdapterError, AdapterErrorCode, OperationControl};
use std::{
    ffi::OsString,
    fs,
    io::Read,
    net::IpAddr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn error(code: AdapterErrorCode, message: &str) -> AdapterError {
    AdapterError::new(code, message, None, true,
        "Review the selected endpoint, independently verified host key and existing SSH key access.")
}

fn executable(name: &str) -> Result<PathBuf, AdapterError> {
    #[cfg(windows)]
    let path = PathBuf::from(r"C:\Program Files\Git\usr\bin").join(format!("{name}.exe"));
    #[cfg(not(windows))]
    let path = PathBuf::from("/usr/bin").join(name);
    if !path.is_file() {
        return Err(error(
            AdapterErrorCode::DependencyMissing,
            "The qualified Git for Windows OpenSSH toolchain is unavailable.",
        ));
    }
    Ok(path)
}

struct ChildOwner(Child);
impl Drop for ChildOwner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Both pipes are drained concurrently with independent byte limits. Raw stderr is discarded.
fn run(
    name: &str,
    args: &[OsString],
    limit: usize,
    control: &OperationControl,
) -> Result<Vec<u8>, AdapterError> {
    control.checkpoint()?;
    let mut command = Command::new(executable(name)?);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
        .env_remove("SSH_ASKPASS")
        .env_remove("SSH_ASKPASS_REQUIRE")
        .env_remove("DISPLAY");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = ChildOwner(command.spawn().map_err(|_| {
        error(
            AdapterErrorCode::TransportError,
            "Could not start the bounded SSH operation.",
        )
    })?);
    let stdout = child
        .0
        .stdout
        .take()
        .ok_or_else(|| error(AdapterErrorCode::TransportError, "SSH output unavailable."))?;
    let stderr = child.0.stderr.take().ok_or_else(|| {
        error(
            AdapterErrorCode::TransportError,
            "SSH diagnostics unavailable.",
        )
    })?;
    std::thread::scope(|scope| {
        let (tx, rx) = mpsc::channel();
        for (index, mut stream) in [
            (0, Box::new(stdout) as Box<dyn Read + Send>),
            (1, Box::new(stderr) as Box<dyn Read + Send>),
        ] {
            let tx = tx.clone();
            scope.spawn(move || {
                let mut data = Vec::new();
                let result = stream
                    .by_ref()
                    .take((limit + 1) as u64)
                    .read_to_end(&mut data);
                let _ = tx.send((index, result.is_ok() && data.len() <= limit, data));
            });
        }
        drop(tx);
        let result = (|| {
            let mut outputs: [Option<Vec<u8>>; 2] = [None, None];
            loop {
                control.checkpoint()?;
                while let Ok((index, valid, data)) = rx.try_recv() {
                    if !valid {
                        return Err(error(
                            AdapterErrorCode::TransportError,
                            "SSH output exceeded its bound or could not be read.",
                        ));
                    }
                    outputs[index] = Some(data);
                }
                let status = child.0.try_wait().map_err(|_| {
                    error(
                        AdapterErrorCode::TransportError,
                        "SSH process state unavailable.",
                    )
                })?;
                if let Some(status) = status {
                    if outputs.iter().all(Option::is_some) {
                        if !status.success() {
                            // Peer-controlled stderr is never surfaced or logged.
                            let diagnostic =
                                String::from_utf8_lossy(outputs[1].as_deref().unwrap_or_default());
                            if diagnostic.contains("Host key verification failed")
                                || diagnostic.contains("REMOTE HOST IDENTIFICATION HAS CHANGED")
                                || diagnostic.contains("No ED25519 host key is known")
                            {
                                return Err(error(
                                    AdapterErrorCode::EndpointUntrusted,
                                    "SSH host key could not be verified against the selected pin.",
                                ));
                            }
                            if diagnostic.contains("Permission denied")
                                || diagnostic.contains("Load key")
                                || diagnostic.contains("no such identity")
                            {
                                return Err(error(
                                    AdapterErrorCode::AuthenticationFailed,
                                    "The selected SSH key could not authenticate the account.",
                                ));
                            }
                            if diagnostic.contains("Bad configuration option")
                                || diagnostic.contains("unsupported")
                            {
                                return Err(error(AdapterErrorCode::DependencyMissing, "Installed SSH toolchain does not support the required secure options or algorithm."));
                            }
                            return Err(error(
                                AdapterErrorCode::TransportError,
                                "SSH rejected the connection or the allowlisted query failed.",
                            ));
                        }
                        return Ok(outputs[0].take().unwrap_or_default());
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })();
        // Kill before scoped threads join, including timeout, cancellation and output overflow.
        let _ = child.0.kill();
        let _ = child.0.wait();
        result
    })
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct TrustDirectory(PathBuf);
impl TrustDirectory {
    fn create() -> Result<Self, AdapterError> {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "obtainer-ssh-{}-{epoch}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        if path
            .to_string_lossy()
            .chars()
            .any(|c| c == '\'' || c.is_control())
        {
            return Err(error(AdapterErrorCode::DependencyMissing, "Temporary directory cannot be represented safely in the qualified SSH configuration."));
        }
        fs::create_dir(&path).map_err(|_| {
            error(
                AdapterErrorCode::TransportError,
                "Cannot create private SSH trust workspace.",
            )
        })?;
        Ok(Self(path))
    }
}
impl Drop for TrustDirectory {
    fn drop(&mut self) {
        // Only the one public-key file we created; never recursively remove a computed directory.
        let _ = fs::remove_file(self.0.join("known_hosts"));
        let _ = fs::remove_dir(&self.0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostObservation {
    address: IpAddr,
    port: u16,
    public_key: String,
    fingerprint: String,
}
impl HostObservation {
    pub fn address(&self) -> IpAddr {
        self.address
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

fn public_key(line: &str) -> Result<String, AdapterError> {
    let parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() != 2
        || parts[0] != "ssh-ed25519"
        || parts[1].len() != 68
        || !parts[1]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
    {
        return Err(error(
            AdapterErrorCode::EndpointUntrusted,
            "Only a valid Ed25519 host key is accepted.",
        ));
    }
    Ok(format!("{} {}", parts[0], parts[1]))
}

/// Observation only. Caller must obtain target approval before invoking; this grants no trust.
pub fn observe(
    address: IpAddr,
    port: u16,
    control: &OperationControl,
) -> Result<HostObservation, AdapterError> {
    if port == 0 || address.is_unspecified() || address.is_multicast() {
        return Err(error(
            AdapterErrorCode::EndpointUntrusted,
            "Invalid selected SSH endpoint.",
        ));
    }
    let args: Vec<OsString> = [
        "-T".into(),
        "5".into(),
        "-t".into(),
        "ed25519".into(),
        "-p".into(),
        port.to_string().into(),
        address.to_string().into(),
    ]
    .into();
    let bytes = run("ssh-keyscan", &args, 4096, control)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        error(
            AdapterErrorCode::EndpointUntrusted,
            "Malformed host key response.",
        )
    })?;
    let keys: Vec<_> = text
        .lines()
        .filter(|s| !s.starts_with('#') && !s.trim().is_empty())
        .collect();
    if keys.len() != 1 {
        return Err(error(
            AdapterErrorCode::EndpointUntrusted,
            "Expected exactly one Ed25519 host key.",
        ));
    }
    let parts: Vec<_> = keys[0].split_whitespace().collect();
    if parts.len() != 3 {
        return Err(error(
            AdapterErrorCode::EndpointUntrusted,
            "Malformed host key response.",
        ));
    }
    let key = public_key(&format!("{} {}", parts[1], parts[2]))?;
    let directory = TrustDirectory::create()?;
    let file = directory.0.join("known_hosts");
    fs::write(&file, format!("obtainer-target {key}\n")).map_err(|_| {
        error(
            AdapterErrorCode::TransportError,
            "Cannot stage the public host key.",
        )
    })?;
    let bytes = run(
        "ssh-keygen",
        &[
            "-l".into(),
            "-E".into(),
            "sha256".into(),
            "-f".into(),
            file.into_os_string(),
        ],
        1024,
        control,
    )?;
    let output = std::str::from_utf8(&bytes).map_err(|_| {
        error(
            AdapterErrorCode::EndpointUntrusted,
            "Host fingerprint unavailable.",
        )
    })?;
    let fingerprint = output
        .split_whitespace()
        .nth(1)
        .filter(|s| s.starts_with("SHA256:") && s.len() == 50)
        .ok_or_else(|| {
            error(
                AdapterErrorCode::EndpointUntrusted,
                "Host fingerprint unavailable.",
            )
        })?;
    Ok(HostObservation {
        address,
        port,
        public_key: key,
        fingerprint: fingerprint.into(),
    })
}

/// Constructed by backend after independent pin confirmation. No Deserialize or credential contents.
pub struct SshTarget {
    observation: HostObservation,
    source: IpAddr,
    username: String,
    identity_file: PathBuf,
    trust: TrustDirectory,
}
impl SshTarget {
    pub fn observation(&self) -> &HostObservation {
        &self.observation
    }
    pub fn source_address(&self) -> IpAddr {
        self.source
    }
    pub fn confirm(
        observation: HostObservation,
        independent_fingerprint: &str,
        source: IpAddr,
        username: &str,
        identity_file: &Path,
    ) -> Result<Self, AdapterError> {
        if independent_fingerprint != observation.fingerprint
            || source.is_unspecified()
            || source.is_multicast()
            || source.is_ipv4() != observation.address.is_ipv4()
            || username.is_empty()
            || username.len() > 32
            || !username
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || !identity_file.is_absolute()
            || !identity_file.is_file()
        {
            return Err(error(
                AdapterErrorCode::EndpointUntrusted,
                "Endpoint trust, selected source address or key reference is invalid.",
            ));
        }
        public_key(&observation.public_key)?;
        let trust = TrustDirectory::create()?;
        fs::write(
            trust.0.join("known_hosts"),
            format!("obtainer-target {}\n", observation.public_key),
        )
        .map_err(|_| {
            error(
                AdapterErrorCode::TransportError,
                "Cannot stage approved host key.",
            )
        })?;
        Ok(Self {
            observation,
            source,
            username: username.into(),
            identity_file: identity_file.into(),
            trust,
        })
    }
    fn args(&self, query: ReadQuery) -> Vec<OsString> {
        let mut args: Vec<OsString> = ["-F", "none", "-T", "-n"]
            .into_iter()
            .map(Into::into)
            .collect();
        for option in [
            "BatchMode=yes",
            "StrictHostKeyChecking=yes",
            "HostKeyAlias=obtainer-target",
            "HostKeyAlgorithms=ssh-ed25519",
            "IdentitiesOnly=yes",
            "IdentityAgent=none",
            "PreferredAuthentications=publickey",
            "PasswordAuthentication=no",
            "KbdInteractiveAuthentication=no",
            "ForwardAgent=no",
            "ForwardX11=no",
            "ClearAllForwardings=yes",
            "PermitLocalCommand=no",
            "ProxyCommand=none",
            "ProxyJump=none",
            "ControlMaster=no",
            "ControlPath=none",
            "UpdateHostKeys=no",
            "VerifyHostKeyDNS=no",
            "CanonicalizeHostname=no",
            "ConnectionAttempts=1",
            "ConnectTimeout=5",
            "ServerAliveInterval=2",
            "ServerAliveCountMax=1",
        ] {
            args.extend(["-o".into(), option.into()]);
        }
        #[cfg(windows)]
        args.extend(["-o".into(), "GlobalKnownHostsFile=NUL".into()]);
        #[cfg(not(windows))]
        args.extend(["-o".into(), "GlobalKnownHostsFile=/dev/null".into()]);
        args.extend([
            "-o".into(),
            format!(
                "UserKnownHostsFile='{}'",
                self.trust
                    .0
                    .join("known_hosts")
                    .to_string_lossy()
                    .replace('\\', "/")
            )
            .into(),
        ]);
        args.extend([
            "-i".into(),
            self.identity_file
                .to_string_lossy()
                .replace('\\', "/")
                .into(),
            "-b".into(),
            self.source.to_string().into(),
            "-p".into(),
            self.observation.port.to_string().into(),
            "-l".into(),
            self.username.clone().into(),
            self.observation.address.to_string().into(),
            query.command().into(),
        ]);
        args
    }
    pub fn query(
        &self,
        query: ReadQuery,
        control: &OperationControl,
    ) -> Result<Vec<u8>, AdapterError> {
        run("ssh", &self.args(query), query.output_limit(), control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_keys_and_shell_fragments_are_rejected() {
        assert!(public_key("ssh-ed25519 abc\nmalicious").is_err());
        assert!(public_key("ssh-rsa AAAA").is_err());
        assert!(public_key("ssh-ed25519 $(command)").is_err());
    }

    #[test]
    fn pinned_arguments_are_closed_and_public_trust_workspace_is_scoped() {
        let source: IpAddr = "192.0.2.2".parse().unwrap();
        let key_workspace = TrustDirectory::create().unwrap();
        let key = key_workspace.0.join("selected key");
        fs::write(&key, "test reference only; never a usable private key").unwrap();
        let observation = HostObservation {
            address: "192.0.2.1".parse().unwrap(),
            port: 22,
            public_key: format!("ssh-ed25519 {}", "A".repeat(68)),
            fingerprint: format!("SHA256:{}", "A".repeat(43)),
        };
        for account in ["root;reboot", "$(id)", "root\nother", "-oProxyCommand=bad"] {
            assert!(SshTarget::confirm(
                observation.clone(),
                &observation.fingerprint,
                source,
                account,
                &key
            )
            .is_err());
        }
        assert!(SshTarget::confirm(
            observation.clone(),
            "SHA256:unconfirmed",
            source,
            "root",
            &key
        )
        .is_err());
        let target = SshTarget::confirm(
            observation.clone(),
            &observation.fingerprint,
            source,
            "root",
            &key,
        )
        .unwrap();
        let workspace = target.trust.0.clone();
        let args: Vec<_> = target
            .args(ReadQuery::Board)
            .into_iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        for required in [
            "StrictHostKeyChecking=yes",
            "IdentityAgent=none",
            "PasswordAuthentication=no",
            "ProxyCommand=none",
            "ControlMaster=no",
        ] {
            assert!(args.iter().any(|v| v == required));
        }
        assert_eq!(args.last().unwrap(), ReadQuery::Board.command());
        assert!(args
            .iter()
            .any(|v| v.starts_with("UserKnownHostsFile='") && v.ends_with("known_hosts'")));
        assert!(workspace.join("known_hosts").is_file());
        drop(target);
        assert!(!workspace.exists());
        fs::remove_file(key).unwrap();
    }

    #[test]
    fn blackhole_peer_cancellation_and_timeout_release_child_and_pipes() {
        use crate::adapters::{AdapterOperation, Cancellation, Stage};
        use std::net::TcpListener;
        // A loopback TCP sink exercises process teardown only, not protocol compatibility.
        for cancel in [true, false] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let (accepted_tx, accepted_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let sink = std::thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                accepted_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                drop(stream);
            });
            let cancellation = Cancellation::default();
            let control = OperationControl::new(
                cancellation.clone(),
                Stage::Connection,
                AdapterOperation::AuthorizedIdentityProbe,
                Duration::from_millis(if cancel { 5000 } else { 750 }),
            )
            .unwrap();
            let worker =
                std::thread::spawn(move || observe("127.0.0.1".parse().unwrap(), port, &control));
            accepted_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            if cancel {
                cancellation.cancel();
            }
            let error = worker.join().unwrap().unwrap_err();
            // Unblock our owned sink even if the expected error assertion fails.
            release_tx.send(()).unwrap();
            sink.join().unwrap();
            assert_eq!(
                error.code,
                if cancel {
                    AdapterErrorCode::OperationCancelled
                } else {
                    AdapterErrorCode::ConnectionTimeout
                }
            );
            // join completed while the TCP sink still held its socket: child/pipes were drained.
        }
    }

    #[test]
    #[ignore = "Requires the explicitly started disposable OpenWrt VM and console fingerprint"]
    fn genuine_openwrt_windows_ssh_interoperability() {
        use crate::adapters::{AdapterOperation, Cancellation, Stage};
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.tools/phase7-vm");
        let console = fs::read_to_string(directory.join("console-identity.log")).unwrap();
        let independent = console
            .split_whitespace()
            .find(|v| v.starts_with("SHA256:"))
            .unwrap();
        let control = || {
            OperationControl::new(
                Cancellation::default(),
                Stage::Read,
                AdapterOperation::ReadConfiguration,
                Duration::from_secs(15),
            )
            .unwrap()
        };
        let address = "127.0.0.1".parse().unwrap();
        let observed = observe(address, 22270, &control()).unwrap();
        assert_eq!(
            observed.fingerprint, independent,
            "Pin must match trusted guest console, not network observation alone"
        );
        assert!(SshTarget::confirm(
            observed.clone(),
            "SHA256:wrong",
            address,
            "root",
            &directory.join("test-key")
        )
        .is_err());
        let target = SshTarget::confirm(
            observed,
            independent,
            address,
            "root",
            &directory.join("test-key"),
        )
        .unwrap();
        let board =
            super::super::query::Board::parse(&target.query(ReadQuery::Board, &control()).unwrap())
                .unwrap();
        assert_eq!(board.release.version, "25.12.6");
        assert_eq!(board.release.target, "x86/64");
        let network =
            super::super::query::networks(&target.query(ReadQuery::Network, &control()).unwrap())
                .unwrap();
        assert!(!network.is_empty());
        let wifi =
            super::super::query::wireless(&target.query(ReadQuery::Wireless, &control()).unwrap())
                .unwrap();
        assert!(wifi.is_empty(), "QEMU fixture has no physical radio");
        let cancelled = control();
        cancelled.cancellation().cancel();
        assert_eq!(
            target.query(ReadQuery::Board, &cancelled).unwrap_err().code,
            AdapterErrorCode::OperationCancelled
        );
        fs::write(directory.join("rust-inspection.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "board": board, "network": network, "wireless": wifi,
            "transport": "Windows OpenSSH", "hostKeyVerifiedAgainst": "owned QEMU serial console",
            "physicalHardware": false
        })).unwrap()).unwrap();
    }
}
