use super::*;
use std::{
    ffi::c_void,
    mem::size_of,
    net::{SocketAddr, TcpStream},
    ptr,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::GetLastError,
    Networking::{WinHttp::*, WinSock::*},
};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn cancelled(token: &AtomicBool) -> Result<(), Box<EndpointResult>> {
    if token.load(Ordering::Acquire) {
        Err(EndpointResult::failure(
            "cancelled",
            "Check cancelled. Previous observation retained.",
        ))
    } else {
        Ok(())
    }
}
fn resolve(host: &str) -> Result<Vec<IpAddr>, Box<EndpointResult>> {
    if let Ok(ip) = host.parse() {
        return Ok(vec![ip]);
    }
    let mut data = WSADATA::default();
    if unsafe { WSAStartup(0x202, &mut data) } != 0 {
        return Err(EndpointResult::failure(
            "dns_error",
            "Windows name resolution could not start.",
        ));
    }
    struct Winsock;
    impl Drop for Winsock {
        fn drop(&mut self) {
            unsafe {
                WSACleanup();
            }
        }
    }
    let _winsock = Winsock;
    let host = wide(host);
    let hints = ADDRINFOEXW {
        ai_family: AF_UNSPEC as i32,
        ai_socktype: SOCK_STREAM,
        ..Default::default()
    };
    let timeout = TIMEVAL {
        tv_sec: 1,
        tv_usec: 0,
    };
    struct DnsEvent(WSAEVENT);
    impl Drop for DnsEvent {
        fn drop(&mut self) {
            unsafe {
                WSACloseEvent(self.0);
            }
        }
    }
    let event = DnsEvent(unsafe { WSACreateEvent() });
    if event.0 == WSA_INVALID_EVENT {
        return Err(EndpointResult::failure(
            "dns_error",
            "Windows could not create a name-resolution event.",
        ));
    }
    let event_handle = event.0 as windows_sys::Win32::Foundation::HANDLE;
    let mut overlapped = windows_sys::Win32::System::IO::OVERLAPPED {
        hEvent: event_handle,
        ..Default::default()
    };
    let mut cancel_handle = ptr::null_mut();
    let mut result = ptr::null_mut();
    let mut code = unsafe {
        GetAddrInfoExW(
            host.as_ptr(),
            ptr::null(),
            NS_DNS,
            ptr::null(),
            &hints,
            &mut result,
            &timeout,
            ptr::addr_of_mut!(overlapped),
            None,
            &mut cancel_handle,
        )
    };
    if code == WSA_IO_PENDING {
        let wait = unsafe { WSAWaitForMultipleEvents(1, &event_handle, 1, 1500, 0) };
        if wait != WSA_WAIT_EVENT_0 as u32 {
            unsafe {
                GetAddrInfoExCancel(&cancel_handle);
                // Cancellation completes asynchronously. Keep every borrowed buffer
                // and OVERLAPPED alive until Windows acknowledges completion.
                WSAWaitForMultipleEvents(1, &event_handle, 1, u32::MAX, 0);
            }
            code = WSAETIMEDOUT;
        } else {
            code = unsafe { GetAddrInfoExOverlappedResult(&overlapped) };
        }
    }
    struct Answers(*mut ADDRINFOEXW);
    impl Drop for Answers {
        fn drop(&mut self) {
            unsafe {
                FreeAddrInfoExW(self.0);
            }
        }
    }
    let answer = Answers(result);
    if code != 0 {
        return Err(EndpointResult::failure(
            if code == WSAETIMEDOUT {
                "timeout"
            } else {
                "dns_error"
            },
            "Hostname resolution failed or timed out. Try the router's local IP address.",
        ));
    }
    let mut ips = vec![];
    let mut current = answer.0;
    for _ in 0..64 {
        if current.is_null() {
            break;
        }
        unsafe {
            let row = &*current;
            if !row.ai_addr.is_null() {
                if row.ai_family == AF_INET as i32 && row.ai_addrlen >= size_of::<SOCKADDR_IN>() {
                    ips.push(IpAddr::from(
                        (*row.ai_addr.cast::<SOCKADDR_IN>())
                            .sin_addr
                            .S_un
                            .S_addr
                            .to_ne_bytes(),
                    ));
                } else if row.ai_family == AF_INET6 as i32
                    && row.ai_addrlen >= size_of::<SOCKADDR_IN6>()
                {
                    ips.push(IpAddr::from(
                        (*row.ai_addr.cast::<SOCKADDR_IN6>()).sin6_addr.u.Byte,
                    ));
                }
            }
            current = row.ai_next;
        }
    }
    ips.sort();
    ips.dedup();
    if ips.is_empty() {
        return Err(EndpointResult::failure(
            "dns_error",
            "Hostname returned no usable IP addresses.",
        ));
    }
    // Mixed public/private answers are rejected; WinHTTP receives one pinned numeric result.
    if ips.iter().any(|ip| !local_address(*ip)) {
        return Err(EndpointResult::failure(
            "target_out_of_scope",
            "Hostname resolves outside permitted local addresses. Use a local router address.",
        ));
    }
    Ok(ips)
}

pub fn probe(
    target: &Target,
    scope: Option<&str>,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
) -> EndpointResult {
    probe_kind(target, scope, networks, token, false).0
}

pub(super) fn probe_root(
    target: &Target,
    scope: Option<&str>,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
) -> (EndpointResult, Option<Vec<u8>>) {
    probe_kind(target, scope, networks, token, true)
}

fn probe_kind(
    target: &Target,
    scope: Option<&str>,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
    root: bool,
) -> (EndpointResult, Option<Vec<u8>>) {
    probe_path(target, scope, networks, token, root, "/", None)
}

pub(super) fn probe_version(
    target: &Target,
    scope: Option<&str>,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
) -> (EndpointResult, Option<Vec<u8>>) {
    probe_path(
        target,
        scope,
        networks,
        token,
        true,
        "/version_num.txt",
        None,
    )
}

pub(super) fn probe_menu(
    target: &Target,
    scope: Option<&str>,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
    remaining: Duration,
) -> (EndpointResult, Option<Vec<u8>>) {
    probe_path(
        target,
        scope,
        networks,
        token,
        true,
        "/xml/ui_mifi.xml",
        Some(Instant::now() + remaining.min(Duration::from_secs(5))),
    )
}
pub(super) fn probe_challenge(
    target: &Target,
    scope: Option<&str>,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
    remaining: Duration,
) -> (EndpointResult, Option<Vec<u8>>) {
    probe_path(
        target,
        scope,
        networks,
        token,
        true,
        "/login.cgi",
        Some(Instant::now() + remaining.min(Duration::from_secs(5))),
    )
}

fn probe_path(
    target: &Target,
    scope: Option<&str>,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
    root: bool,
    path: &str,
    menu_deadline: Option<Instant>,
) -> (EndpointResult, Option<Vec<u8>>) {
    let start = Instant::now();
    let work = || -> Result<(EndpointResult, Option<Vec<u8>>), Box<EndpointResult>> {
        cancelled(token)?;
        let ips = resolve(&target.host)?;
        cancelled(token)?;
        // Choose one on-link address, not a list of fallback destinations to scan.
        let mut choices = vec![];
        let mut reason = None;
        for ip in ips {
            match select_interface(ip, scope, networks) {
                Ok(n) => choices.push((ip, n)),
                Err(e) => reason = Some(e),
            }
        }
        let (ip, network) = choices.first().copied().ok_or_else(|| {
            reason.unwrap_or_else(|| EndpointResult::failure("no_route", "No route to router."))
        })?;
        if root {
            if let Some(conflict) = subnet_conflict(ip, network, networks) {
                return Err(conflict);
            }
        }
        if root
            && networks
                .iter()
                .any(|n| n.addresses.iter().any(|a| bare_ip(a) == Some(ip)))
        {
            return Err(EndpointResult::failure(
                "host_endpoint",
                "The selected endpoint belongs to this PC, not a router.",
            ));
        }
        let index = if ip.is_ipv6() {
            network.ipv6_index
        } else {
            network.interface_index
        };
        let socket = match ip {
            IpAddr::V4(v) => SocketAddr::new(v.into(), target.port),
            IpAddr::V6(v) => SocketAddr::V6(std::net::SocketAddrV6::new(
                v,
                target.port,
                0,
                if v.is_unicast_link_local() { index } else { 0 },
            )),
        };
        // Confirm the system will actually use the selected interface. This also prevents
        // a more-specific VPN route from silently replacing the associated LAN route.
        let destination = sockaddr(socket);
        let mut best =
            windows_sys::Win32::NetworkManagement::IpHelper::MIB_IPFORWARD_ROW2::default();
        let mut source = SOCKADDR_INET::default();
        let code = unsafe {
            windows_sys::Win32::NetworkManagement::IpHelper::GetBestRoute2(
                ptr::null(),
                0,
                ptr::null(),
                &destination,
                0,
                &mut best,
                &mut source,
            )
        };
        if code != 0 || best.InterfaceIndex != index {
            return Err(EndpointResult::failure("no_route", "Windows would use a different interface for this target. Check routing or VPN settings, then refresh."));
        }
        cancelled(token)?;
        let tcp_budget = menu_deadline
            .map(|d| {
                d.saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(800))
            })
            .unwrap_or(Duration::from_millis(800));
        if tcp_budget.is_zero() {
            return Err(EndpointResult::failure(
                "timeout",
                "Static menu deadline exceeded.",
            ));
        }
        // A single TCP preflight distinguishes refusal from HTTP/TLS transport errors.
        match TcpStream::connect_timeout(&socket, tcp_budget) {
            Ok(stream) => drop(stream),
            Err(e) => return Err(EndpointResult::failure(match e.kind() { std::io::ErrorKind::ConnectionRefused => "connection_refused", std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => "timeout", _ => "unreachable" }, "Router management port did not accept the bounded connection. Check address, port and connection.")),
        }
        cancelled(token)?;
        let pinned = if matches!(ip, IpAddr::V6(v) if v.is_unicast_link_local()) {
            format!("{ip}%{index}")
        } else {
            ip.to_string()
        };
        let (mut result, body) =
            request_path_inner(target, &pinned, token, root, path, menu_deadline)?;
        result.resolved_address = Some(pinned);
        Ok((result, body))
    };
    let (mut result, mut body) = work().unwrap_or_else(|e| (*e, None));
    if token.load(Ordering::Acquire) {
        result = *EndpointResult::failure(
            "cancelled",
            "Check cancelled. Previous observation retained.",
        );
        body = None;
    }
    result.elapsed_ms = start.elapsed().as_millis() as u64;
    if matches!(
        result.code.as_str(),
        "no_route" | "connection_refused" | "timeout" | "unreachable"
    ) {
        result.state = "unreachable".into();
    }
    (result, body)
}
fn sockaddr(socket: SocketAddr) -> SOCKADDR_INET {
    let mut out = SOCKADDR_INET::default();
    match socket {
        SocketAddr::V4(s) => {
            out.Ipv4.sin_family = AF_INET;
            out.Ipv4.sin_port = s.port().to_be();
            out.Ipv4.sin_addr.S_un.S_addr = u32::from_ne_bytes(s.ip().octets());
        }
        SocketAddr::V6(s) => {
            out.Ipv6.sin6_family = AF_INET6;
            out.Ipv6.sin6_port = s.port().to_be();
            out.Ipv6.sin6_addr.u.Byte = s.ip().octets();
            out.Ipv6.Anonymous.sin6_scope_id = s.scope_id();
        }
    }
    out
}

pub(super) fn binding_route(
    target: &Target,
    network: &NetworkAdapter,
) -> Result<crate::endpoint_binding::Route, crate::endpoint_binding::BindingError> {
    use crate::endpoint_binding::{BindingError, Route};
    let ip: IpAddr = target.host.parse().map_err(|_| BindingError)?;
    let index = if ip.is_ipv6() {
        network.ipv6_index
    } else {
        network.interface_index
    };
    let socket = match ip {
        IpAddr::V4(v) => SocketAddr::from((v, target.port)),
        IpAddr::V6(v) => SocketAddr::V6(std::net::SocketAddrV6::new(
            v,
            target.port,
            0,
            if v.is_unicast_link_local() { index } else { 0 },
        )),
    };
    let destination = sockaddr(socket);
    let mut best = windows_sys::Win32::NetworkManagement::IpHelper::MIB_IPFORWARD_ROW2::default();
    let mut source = SOCKADDR_INET::default();
    let code = unsafe {
        windows_sys::Win32::NetworkManagement::IpHelper::GetBestRoute2(
            ptr::null(),
            0,
            ptr::null(),
            &destination,
            0,
            &mut best,
            &mut source,
        )
    };
    if code != 0 || index == 0 || best.InterfaceIndex != index {
        return Err(BindingError);
    }
    fn address(value: &SOCKADDR_INET) -> Option<String> {
        unsafe {
            match value.si_family {
                AF_INET => {
                    Some(IpAddr::from(value.Ipv4.sin_addr.S_un.S_addr.to_ne_bytes()).to_string())
                }
                AF_INET6 => {
                    let ip = std::net::Ipv6Addr::from(value.Ipv6.sin6_addr.u.Byte);
                    let scope = value.Ipv6.Anonymous.sin6_scope_id;
                    Some(if scope == 0 {
                        ip.to_string()
                    } else {
                        format!("{ip}%{scope}")
                    })
                }
                _ => None,
            }
        }
    }
    let gateway = address(&best.NextHop).filter(|v| v != "0.0.0.0" && v != "::");
    Ok(Route {
        source: address(&source).ok_or(BindingError)?,
        gateway,
        luid: unsafe { best.InterfaceLuid.Value },
    })
}

struct Handle(*mut c_void);
impl Handle {
    fn checked(value: *mut c_void) -> Result<Self, Box<EndpointResult>> {
        if value.is_null() {
            Err(http_error())
        } else {
            Ok(Self(value))
        }
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            WinHttpCloseHandle(self.0);
        }
    }
}
fn http_error() -> Box<EndpointResult> {
    windows_error(unsafe { GetLastError() })
}
fn windows_error(code: u32) -> Box<EndpointResult> {
    tracing::debug!(windows_error = code, "Read-only endpoint transport failed");
    #[cfg(test)]
    eprintln!("WinHTTP error {code}");
    let (key, message) = match code {
        ERROR_WINHTTP_HEADER_SIZE_OVERFLOW => ("challenge_not_usable", "Response headers exceed the bounded limit."),
        ERROR_WINHTTP_TIMEOUT => ("timeout", "HTTP response timed out. Retry after checking the router connection."),
        ERROR_WINHTTP_SECURE_FAILURE | ERROR_WINHTTP_SECURE_INVALID_CA | ERROR_WINHTTP_SECURE_CERT_CN_INVALID | ERROR_WINHTTP_SECURE_CERT_DATE_INVALID | ERROR_WINHTTP_SECURE_INVALID_CERT | ERROR_WINHTTP_SECURE_CERT_REVOKED => ("tls_trust_failure", "HTTPS certificate validation failed. Verify the router certificate and hostname through its owner before deliberately trusting its issuing certificate in Windows. No validation bypass is applied."),
        ERROR_WINHTTP_CLIENT_AUTH_CERT_NEEDED => ("authentication_required", "The endpoint requires a client certificate. Authentication is unavailable in Phase 2."),
        ERROR_WINHTTP_CANNOT_CONNECT | ERROR_WINHTTP_CONNECTION_ERROR => ("unreachable", "Management connection failed after TCP preflight. Retry the saved address."),
        ERROR_WINHTTP_INVALID_OPTION => ("unsupported_platform", "This Windows version does not support safe hostname pinning. Use an IP address on Windows 10 21H1 or later."),
        _ => ("unsupported_management", "Endpoint did not provide a supported HTTP response."),
    };
    let mut result = EndpointResult::failure(key, message);
    if key == "tls_trust_failure" {
        result.tls = "untrusted".into();
    }
    result
}
// Transient header buffers have no Debug/Serialize implementation and are erased,
// including malformed/oversized/early-return paths. No nonce-owned String is made.
struct ErasedWide(Vec<u16>);
impl Drop for ErasedWide {
    fn drop(&mut self) {
        for n in &mut self.0 {
            unsafe { std::ptr::write_volatile(n, 0) }
        }
        std::sync::atomic::compiler_fence(Ordering::SeqCst);
    }
}
struct ErasedHeader(Vec<u8>);
impl Drop for ErasedHeader {
    fn drop(&mut self) {
        for n in &mut self.0 {
            unsafe { std::ptr::write_volatile(n, 0) }
        }
        std::sync::atomic::compiler_fence(Ordering::SeqCst);
    }
}
fn challenge_headers(request: &Handle, status: u16) -> crate::acquisition::ChallengeObservation {
    use crate::acquisition::{parse_challenge_headers, ChallengeObservation};
    let failure = || ChallengeObservation::header_failure(Some(status), false);
    let mut bytes = 0;
    unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_RAW_HEADERS_CRLF,
            ptr::null(),
            ptr::null_mut(),
            &mut bytes,
            ptr::null_mut(),
        );
    }
    if bytes > 16386 {
        return ChallengeObservation::header_failure(Some(status), true);
    }
    if bytes == 0 {
        return failure();
    }
    let mut raw = ErasedWide(vec![0; (bytes as usize).div_ceil(2)]);
    if unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_RAW_HEADERS_CRLF,
            ptr::null(),
            raw.0.as_mut_ptr().cast(),
            &mut bytes,
            ptr::null_mut(),
        )
    } == 0
    {
        return failure();
    }
    let count = (bytes as usize / 2).min(raw.0.len());
    let mut ascii = ErasedHeader(Vec::with_capacity(count));
    for &n in &raw.0[..count] {
        if n == 0 {
            break;
        }
        if n > 127 {
            return failure();
        }
        ascii.0.push(n as u8);
    }
    parse_challenge_headers(status, &ascii.0)
}
fn set(handle: &Handle, key: u32, value: u32) -> Result<(), Box<EndpointResult>> {
    if unsafe { WinHttpSetOption(handle.0, key, (&value as *const u32).cast(), 4) } == 0 {
        return Err(http_error());
    }
    Ok(())
}
struct CallbackContext {
    sender: std::sync::mpsc::SyncSender<Result<u32, u32>>,
    body_sender: std::sync::mpsc::SyncSender<Result<Vec<u8>, u32>>,
    read_buffer: [u8; 8192],
}
unsafe extern "system" fn callback(
    _handle: *mut c_void,
    context: usize,
    status: u32,
    info: *mut c_void,
    length: u32,
) {
    if context == 0 {
        return;
    }
    if status == WINHTTP_CALLBACK_STATUS_HANDLE_CLOSING {
        // Microsoft guarantees this is the last callback for the request handle.
        drop(Box::from_raw(context as *mut CallbackContext));
        return;
    }
    let context = &*(context as *const CallbackContext);
    if status == WINHTTP_CALLBACK_STATUS_READ_COMPLETE
        && length <= 8192
        && (!info.is_null() || length == 0)
    {
        let bytes = if length == 0 {
            vec![]
        } else {
            std::slice::from_raw_parts(info.cast::<u8>(), length as usize).to_vec()
        };
        let _ = context.body_sender.try_send(Ok(bytes));
    }
    if status == WINHTTP_CALLBACK_STATUS_REQUEST_ERROR
        && !info.is_null()
        && length as usize >= size_of::<WINHTTP_ASYNC_RESULT>()
    {
        let _ = context
            .body_sender
            .try_send(Err((*info.cast::<WINHTTP_ASYNC_RESULT>()).dwError));
    }
    let event = match status {
        WINHTTP_CALLBACK_STATUS_SENDREQUEST_COMPLETE
        | WINHTTP_CALLBACK_STATUS_HEADERS_AVAILABLE => Some(Ok(status)),
        WINHTTP_CALLBACK_STATUS_REQUEST_ERROR
            if !info.is_null() && length as usize >= size_of::<WINHTTP_ASYNC_RESULT>() =>
        {
            Some(Err((*info.cast::<WINHTTP_ASYNC_RESULT>()).dwError))
        }
        _ => None,
    };
    if let Some(event) = event {
        let _ = context.sender.try_send(event);
    }
}
fn wait(
    receiver: &std::sync::mpsc::Receiver<Result<u32, u32>>,
    expected: u32,
    deadline: Instant,
    token: &AtomicBool,
) -> Result<(), Box<EndpointResult>> {
    loop {
        cancelled(token)?;
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(EndpointResult::failure("timeout", "Management check reached its three-second HTTP deadline. Retry after checking the connection."));
        }
        match receiver.recv_timeout(left.min(Duration::from_millis(50))) {
            Ok(Ok(status)) if status == expected => return Ok(()),
            Ok(Err(code)) => return Err(windows_error(code)),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(EndpointResult::failure(
                    "unreachable",
                    "HTTP callback channel closed.",
                ))
            }
            _ => {}
        }
    }
}

#[cfg(test)]
pub(super) fn head(
    target: &Target,
    pinned: &str,
    token: &AtomicBool,
) -> Result<EndpointResult, Box<EndpointResult>> {
    request(target, pinned, token, false).map(|(result, _)| result)
}

#[cfg(test)]
fn request(
    target: &Target,
    pinned: &str,
    token: &AtomicBool,
    root: bool,
) -> Result<(EndpointResult, Option<Vec<u8>>), Box<EndpointResult>> {
    request_path(target, pinned, token, root, "/")
}
#[cfg(test)]
fn request_path(
    target: &Target,
    pinned: &str,
    token: &AtomicBool,
    root: bool,
    path: &str,
) -> Result<(EndpointResult, Option<Vec<u8>>), Box<EndpointResult>> {
    request_path_inner(target, pinned, token, root, path, None)
}
fn request_path_inner(
    target: &Target,
    pinned: &str,
    token: &AtomicBool,
    root: bool,
    path: &str,
    menu_deadline: Option<Instant>,
) -> Result<(EndpointResult, Option<Vec<u8>>), Box<EndpointResult>> {
    cancelled(token)?;
    if menu_deadline.is_some_and(|d| Instant::now() >= d) {
        return Err(EndpointResult::failure(
            "timeout",
            "Static menu deadline exceeded.",
        ));
    }
    let session = Handle::checked(unsafe {
        WinHttpOpen(
            wide("OBTAINER/0.1 read-only discovery").as_ptr(),
            WINHTTP_ACCESS_TYPE_NO_PROXY,
            ptr::null(),
            ptr::null(),
            WINHTTP_FLAG_ASYNC,
        )
    })?;
    if unsafe { WinHttpSetTimeouts(session.0, 1000, 1200, 1000, 1500) } == 0 {
        return Err(http_error());
    }
    set(&session, WINHTTP_OPTION_CONNECT_RETRIES, 0)?;
    let connection = Handle::checked(unsafe {
        WinHttpConnect(session.0, wide(&target.host).as_ptr(), target.port, 0)
    })?;
    let request = Handle::checked(unsafe {
        WinHttpOpenRequest(
            connection.0,
            wide(if root { "GET" } else { "HEAD" }).as_ptr(),
            wide(path).as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            if target.secure {
                WINHTTP_FLAG_SECURE
            } else {
                0
            },
        )
    })?;
    if unsafe { WinHttpSetTimeouts(request.0, 1000, 1200, 1000, 1500) } == 0 {
        return Err(http_error());
    }
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
    set(
        &request,
        WINHTTP_OPTION_MAX_RESPONSE_HEADER_SIZE,
        if path == "/login.cgi" { 8192 } else { 16384 },
    )?;
    set(&request, WINHTTP_OPTION_RECEIVE_RESPONSE_TIMEOUT, 1500)?;
    // Preserve original Host and TLS name while resolving only the validated numeric IP.
    let pinned = wide(pinned);
    if unsafe {
        WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_RESOLUTION_HOSTNAME,
            pinned.as_ptr().cast(),
            (pinned.len() * 2) as u32,
        )
    } == 0
    {
        return Err(http_error());
    }
    let deadline = menu_deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(3));
    let (sender, receiver) = std::sync::mpsc::sync_channel(4);
    let (body_sender, body_receiver) = std::sync::mpsc::sync_channel(2);
    let mut context = Box::new(CallbackContext {
        sender,
        body_sender,
        read_buffer: [0; 8192],
    });
    let read_pointer = context.read_buffer.as_mut_ptr();
    let previous = unsafe {
        WinHttpSetStatusCallback(
            request.0,
            Some(callback),
            WINHTTP_CALLBACK_FLAG_ALL_NOTIFICATIONS,
            0,
        )
    };
    if previous.is_some_and(|f| f as usize == usize::MAX) {
        return Err(http_error());
    }
    let context_pointer = (&mut *context as *mut CallbackContext) as usize;
    if unsafe {
        WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_CONTEXT_VALUE,
            (&context_pointer as *const usize).cast(),
            size_of::<usize>() as u32,
        )
    } == 0
    {
        return Err(http_error());
    }
    // The final HANDLE_CLOSING callback owns/frees this context, including on cancellation.
    let context_pointer = Box::into_raw(context) as usize;
    cancelled(token)?;
    if Instant::now() >= deadline {
        return Err(EndpointResult::failure(
            "timeout",
            "Static menu deadline exceeded.",
        ));
    }
    if unsafe {
        WinHttpSendRequest(
            request.0,
            ptr::null(),
            0,
            ptr::null(),
            0,
            0,
            context_pointer,
        )
    } == 0
    {
        return Err(http_error());
    }
    wait(
        &receiver,
        WINHTTP_CALLBACK_STATUS_SENDREQUEST_COMPLETE,
        deadline,
        token,
    )?;
    if unsafe { WinHttpReceiveResponse(request.0, ptr::null_mut()) } == 0 {
        return Err(http_error());
    }
    wait(
        &receiver,
        WINHTTP_CALLBACK_STATUS_HEADERS_AVAILABLE,
        deadline,
        token,
    )?;
    let mut status = 0u32;
    let mut length = 4u32;
    if unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            (&mut status as *mut u32).cast(),
            &mut length,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(http_error());
    }
    if path == "/login.cgi" {
        cancelled(token)?;
        let observation = challenge_headers(&request, status as u16);
        cancelled(token)?;
        if Instant::now() >= deadline {
            return Err(EndpointResult::failure(
                "timeout",
                "Challenge deadline exceeded.",
            ));
        }
        let mut result = *EndpointResult::failure(
            &observation.code,
            "Credential-free challenge check finished.",
        );
        result.status = Some(status as u16);
        result.auth_challenge = Some(observation);
        // Never query a response body, Server, cookies or authentication state here.
        return Ok((result, None));
    }
    let mut buffer = [0u16; 256];
    let mut length = 510u32;
    let server = if unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_SERVER,
            ptr::null(),
            buffer.as_mut_ptr().cast(),
            &mut length,
            ptr::null_mut(),
        )
    } != 0
    {
        Some(
            String::from_utf16_lossy(&buffer[..(length as usize / 2).min(255)])
                .trim_end_matches('\0')
                .chars()
                .filter(|c| !c.is_control())
                .collect(),
        )
    } else {
        None
    };
    let (code, state, message) = match status {
        401 | 403 => ("authentication_required", "authentication_required", "Endpoint requires or denies access. No credentials were sent; detailed authentication belongs to Phase 5."),
        200..=299 => ("identity_unknown", "reachable", "HTTP endpoint responded. Router identity, model and firmware remain unverified."),
        300..=399 => ("redirect_blocked", "reachable", "Endpoint returned a redirect. It was not followed. Enter the intended local destination explicitly if authorized."),
        _ => ("unsupported_management", "identity_unknown", "HTTP endpoint responded but this read-only HEAD check cannot identify its management interface."),
    };
    if menu_deadline.is_some() && (200..300).contains(&status) {
        let mut content_type = [0u16; 256];
        let mut size = 510u32;
        if unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_CONTENT_TYPE,
                ptr::null(),
                content_type.as_mut_ptr().cast(),
                &mut size,
                ptr::null_mut(),
            )
        } == 0
        {
            let mut failure =
                EndpointResult::failure("wrong_content_type", "Static menu did not return XML.");
            failure.observed_media_type = Some(menu_media_type(None));
            return Err(failure);
        }
        let mime = String::from_utf16_lossy(&content_type[..(size as usize / 2).min(255)]);
        let mime = mime
            .trim_end_matches('\0')
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        if !mime.eq_ignore_ascii_case("application/xml") && !mime.eq_ignore_ascii_case("text/xml") {
            let mut failure =
                EndpointResult::failure("wrong_content_type", "Static menu did not return XML.");
            failure.observed_media_type = Some(menu_media_type(Some(&mime.to_ascii_lowercase())));
            return Err(failure);
        }
    }
    let mut declared_length = 0u32;
    let mut declared_size = 4u32;
    let expected_length = if menu_deadline.is_some()
        && (200..300).contains(&status)
        && unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_CONTENT_LENGTH | WINHTTP_QUERY_FLAG_NUMBER,
                ptr::null(),
                (&mut declared_length as *mut u32).cast(),
                &mut declared_size,
                ptr::null_mut(),
            )
        } != 0
    {
        Some(declared_length as usize)
    } else {
        None
    };
    if expected_length.is_some_and(|n| n > 16384) {
        return Err(EndpointResult::failure(
            "metadata_too_large",
            "Static menu exceeds 16 KiB.",
        ));
    }
    let mut body = None;
    if root && (200..300).contains(&status) {
        let mut bytes = Vec::new();
        while bytes.len() < 16384 || (menu_deadline.is_some() && bytes.len() == 16384) {
            cancelled(token)?;
            if unsafe { WinHttpReadData(request.0, read_pointer.cast(), 8192, ptr::null_mut()) }
                == 0
            {
                return Err(http_error());
            }
            let chunk = loop {
                cancelled(token)?;
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return Err(EndpointResult::failure(
                        "timeout",
                        "Root metadata deadline exceeded.",
                    ));
                }
                match body_receiver.recv_timeout(left.min(Duration::from_millis(50))) {
                    Ok(Ok(chunk)) => break chunk,
                    Ok(Err(error)) => return Err(windows_error(error)),
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        return Err(EndpointResult::failure(
                            "unreachable",
                            "Root metadata channel closed.",
                        ))
                    }
                    _ => {}
                }
            };
            if chunk.is_empty() {
                break;
            }
            if menu_deadline.is_some() && bytes.len() + chunk.len() > 16384 {
                return Err(EndpointResult::failure(
                    "metadata_too_large",
                    "Static menu exceeds 16 KiB.",
                ));
            }
            bytes.extend_from_slice(&chunk[..chunk.len().min(16384 - bytes.len())]);
        }
        if expected_length.is_some_and(|n| n != bytes.len()) {
            return Err(EndpointResult::failure(
                "truncated_metadata",
                "Static menu response was truncated.",
            ));
        }
        body = Some(bytes);
    }
    Ok((
        EndpointResult {
            code: code.into(),
            state: state.into(),
            message: message.into(),
            status: Some(status as u16),
            server,
            observed_media_type: None,
            auth_challenge: None,
            checked_at: now(),
            elapsed_ms: 0,
            resolved_address: None,
            tls: if target.secure {
                "validated"
            } else {
                "not_used"
            }
            .into(),
        },
        body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    #[test]
    fn cancellation_during_root_body_read_releases_callback_buffers() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (started, ready) = std::sync::mpsc::channel();
        let (release, stop) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0; 4096];
            let _ = stream.read(&mut bytes).unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 500\r\nConnection: close\r\n\r\n")
                .unwrap();
            started.send(()).unwrap();
            let _ = stop.recv_timeout(Duration::from_secs(5));
        });
        let token = std::sync::Arc::new(AtomicBool::new(false));
        let worker_token = token.clone();
        let worker = std::thread::spawn(move || {
            request(
                &Target {
                    url: String::new(),
                    host: "localhost".into(),
                    port,
                    secure: false,
                },
                "127.0.0.1",
                &worker_token,
                true,
            )
        });
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        token.store(true, Ordering::Release);
        assert_eq!(worker.join().unwrap().unwrap_err().code, "cancelled");
        release.send(()).unwrap();
        server.join().unwrap();
    }
    #[test]
    fn root_metadata_is_one_bounded_credential_free_get_and_redirects_are_not_followed() {
        for (status, body) in [
            (
                "200 OK",
                "<title>E-LINK Wireless Router</title>".to_string(),
            ),
            ("302 Found", "redirect".to_string()),
            ("401 Unauthorized", "login".to_string()),
            ("200 OK", "x".repeat(20000)),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let expected = body.as_bytes()[..body.len().min(16384)].to_vec();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let sent = String::from_utf8_lossy(&buffer[..count]);
                assert!(sent.starts_with("GET / HTTP/1.1"));
                assert!(!sent.to_ascii_lowercase().contains("authorization:"));
                assert!(!sent.to_ascii_lowercase().contains("cookie:"));
                let _=stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nLocation: http://example.com/forbidden\r\nSet-Cookie: private=secret\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes());
            });
            let target = Target {
                url: String::new(),
                host: "localhost".into(),
                port,
                secure: false,
            };
            let (result, bytes) =
                request(&target, "127.0.0.1", &AtomicBool::new(false), true).unwrap();
            if status == "200 OK" {
                assert_eq!(bytes.unwrap(), expected);
            } else {
                assert!(bytes.is_none());
                assert!(matches!(
                    result.code.as_str(),
                    "redirect_blocked" | "authentication_required"
                ));
            }
            server.join().unwrap();
        }
    }
    #[test]
    fn static_version_request_is_fixed_credential_free_and_bounded() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 4096];
            let count = stream.read(&mut buffer).unwrap();
            let sent = String::from_utf8_lossy(&buffer[..count]);
            assert!(sent.starts_with("GET /version_num.txt HTTP/1.1"));
            assert!(!sent.to_ascii_lowercase().contains("authorization:"));
            assert!(!sent.to_ascii_lowercase().contains("cookie:"));
            let body = "<firmware_version>REL_MRVL_UAPXC_v2.6.0</firmware_version>";
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .unwrap();
        });
        let target = Target {
            url: String::new(),
            host: "localhost".into(),
            port,
            secure: false,
        };
        let (result, body) = request_path(
            &target,
            "127.0.0.1",
            &AtomicBool::new(false),
            true,
            "/version_num.txt",
        )
        .unwrap();
        assert_eq!(result.status, Some(200));
        assert!(body.unwrap().starts_with(b"<firmware_version>"));
        server.join().unwrap();
    }
    #[test]
    fn review_hostname_resolution_supports_localhost() {
        // Resolution must finish, then the existing endpoint policy rejects loopback.
        assert_eq!(
            resolve("localhost").unwrap_err().code,
            "target_out_of_scope"
        );
        assert_eq!(
            resolve("192.168.100.1").unwrap(),
            vec!["192.168.100.1".parse::<IpAddr>().unwrap()]
        );
    }
    #[test]
    fn local_http_checks_auth_redirect_unknown_and_timeout_without_credentials() {
        for (response, expected) in [
            ("200 OK", "identity_unknown"),
            ("401 Unauthorized", "authentication_required"),
            ("302 Found", "redirect_blocked"),
            ("405 Method Not Allowed", "unsupported_management"),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]);
                assert!(request.starts_with("HEAD / HTTP/1.1"));
                assert!(!request.to_lowercase().contains("authorization:"));
                stream.write_all(format!("HTTP/1.1 {response}\r\nLocation: http://example.com/\r\nServer: fixture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
            });
            let target = Target {
                url: String::new(),
                host: "localhost".into(),
                port,
                secure: false,
            };
            assert_eq!(
                head(&target, "127.0.0.1", &AtomicBool::new(false))
                    .unwrap()
                    .code,
                expected
            );
            server.join().unwrap();
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let target = Target {
            url: String::new(),
            host: "localhost".into(),
            port: listener.local_addr().unwrap().port(),
            secure: false,
        };
        let server = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(3500));
        });
        let start = Instant::now();
        assert_eq!(
            head(&target, "127.0.0.1", &AtomicBool::new(false))
                .unwrap_err()
                .code,
            "timeout"
        );
        assert!(start.elapsed() < Duration::from_secs(4));
        server.join().unwrap();
        assert_eq!(
            head(&target, "127.0.0.1", &AtomicBool::new(true))
                .unwrap_err()
                .code,
            "cancelled"
        );
    }

    #[test]
    fn in_progress_http_can_be_cancelled_before_its_deadline() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let target = Target {
            url: String::new(),
            host: "localhost".into(),
            port: listener.local_addr().unwrap().port(),
            secure: false,
        };
        let token = Arc::new(AtomicBool::new(false));
        let cancel = Arc::clone(&token);
        let server = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            cancel.store(true, Ordering::Release);
            std::thread::sleep(Duration::from_millis(400));
        });
        let start = Instant::now();
        assert_eq!(
            head(&target, "127.0.0.1", &token).unwrap_err().code,
            "cancelled"
        );
        assert!(start.elapsed() < Duration::from_millis(350));
        server.join().unwrap();
    }
    fn synthetic_menu_request(stream: &mut std::net::TcpStream) -> String {
        // Windows accepted sockets may inherit the listener's nonblocking mode.
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = [0; 4096];
        let mut used = 0;
        while used < bytes.len() {
            let count = stream.read(&mut bytes[used..]).unwrap();
            assert!(count > 0, "synthetic peer closed before request headers");
            used += count;
            if bytes[..used].windows(4).any(|w| w == b"\r\n\r\n") {
                return String::from_utf8(bytes[..used].to_vec()).unwrap();
            }
        }
        panic!("synthetic request headers exceeded bound")
    }

    #[test]
    fn menu_loopback_is_one_xml_get_and_enforces_stop_rules() {
        // All bodies are synthetic; listener exists before client, with bounded accept/read.
        for (status, mime, body, expected) in [
            (
                "200 OK",
                "application/xml; charset=UTF-8",
                "<synthetic/>",
                "identity_unknown",
            ),
            (
                "401 Unauthorized",
                "text/xml",
                "<synthetic/>",
                "authentication_required",
            ),
            ("302 Found", "text/xml", "<synthetic/>", "redirect_blocked"),
            ("200 OK", "text/html", "<synthetic/>", "wrong_content_type"),
            ("200 OK", "text/plain", "<synthetic/>", "wrong_content_type"),
            ("200 OK", "", "<synthetic/>", "wrong_content_type"),
            (
                "200 OK",
                "text/html/SECRET@host",
                "<synthetic/>",
                "wrong_content_type",
            ),
            (
                "200 OK",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "<synthetic/>",
                "wrong_content_type",
            ),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            listener.set_nonblocking(true).unwrap();
            let server = std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((s, _)) => break s,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            std::thread::park_timeout(Duration::from_millis(5))
                        }
                        Err(e) => panic!("synthetic peer: {e}"),
                    }
                };
                let sent = synthetic_menu_request(&mut stream);
                assert!(sent.starts_with("GET /xml/ui_mifi.xml HTTP/1.1"));
                assert!(!sent.to_ascii_lowercase().contains("authorization:"));
                assert!(!sent.to_ascii_lowercase().contains("cookie:"));
                let header = if mime.is_empty() {
                    String::new()
                } else {
                    format!("Content-Type: {mime}\r\n")
                };
                let _=stream.write_all(format!("HTTP/1.1 {status}\r\n{header}Content-Length: {}\r\nLocation: /must-not-follow\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes());
            });
            let target = Target {
                url: String::new(),
                host: "localhost".into(),
                port,
                secure: false,
            };
            let outcome = request_path_inner(
                &target,
                "127.0.0.1",
                &AtomicBool::new(false),
                true,
                "/xml/ui_mifi.xml",
                Some(Instant::now() + Duration::from_secs(5)),
            );
            let code = match outcome {
                Ok((r, b)) => {
                    if expected != "identity_unknown" {
                        assert!(b.is_none())
                    };
                    r.code
                }
                Err(r) => {
                    if expected == "wrong_content_type" {
                        let media = if matches!(mime, "text/html" | "text/plain") {
                            mime
                        } else {
                            "unavailable"
                        };
                        assert_eq!(
                            serde_json::to_value(&r).unwrap()["observedMediaType"],
                            media
                        );
                    }
                    r.code
                }
            };
            assert_eq!(code, expected);
            server.join().unwrap();
        }
    }
    #[test]
    fn challenge_loopback_headers_only_single_credential_free_get() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let end = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < end =>
                    {
                        std::thread::park_timeout(Duration::from_millis(5))
                    }
                    Err(e) => panic!("synthetic peer {e}"),
                }
            };
            let request = synthetic_menu_request(&mut stream);
            assert!(request.starts_with("GET /login.cgi HTTP/1.1\r\n"));
            let lower = request.to_ascii_lowercase();
            assert!(
                !lower.contains("authorization:")
                    && !lower.contains("cookie:")
                    && !lower.contains("/login.cgi?")
            );
            stream
                .set_write_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let _=stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Digest realm=\"Synthetic\", nonce=\"d00df00d1234abcd\", qop=\"auth\"\r\nSet-Cookie: synthetic=private\r\nContent-Length: 10000000\r\nConnection: close\r\n\r\n");
            // Advertise a huge body, supply only a small prefix. A body-reading client
            // would wait for the missing remainder; a header-only client closes now.
            let _ = stream.write_all(&vec![b'x'; 32768]);
        });
        let target = Target {
            url: String::new(),
            host: "localhost".into(),
            port,
            secure: false,
        };
        let start = Instant::now();
        let (result, body) = request_path_inner(
            &target,
            "127.0.0.1",
            &AtomicBool::new(false),
            true,
            "/login.cgi",
            Some(Instant::now() + Duration::from_secs(5)),
        )
        .unwrap();
        server.join().unwrap();
        assert!(body.is_none());
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["authChallenge"]["code"], "observed");
        assert_eq!(json["authChallenge"]["setCookiePresent"], true);
        assert!(!json.to_string().contains("d00df00d1234abcd"));
        assert!(start.elapsed() < Duration::from_secs(5));
    }
    #[test]
    fn challenge_loopback_cancellation_and_timeout_release_connection() {
        for cancel in [true, false] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            listener.set_nonblocking(true).unwrap();
            let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
            let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
            let server = std::thread::spawn(move || {
                let end = Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((s, _)) => break s,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < end =>
                        {
                            std::thread::park_timeout(Duration::from_millis(5))
                        }
                        Err(e) => panic!("synthetic peer {e}"),
                    }
                };
                let request = synthetic_menu_request(&mut stream);
                assert!(request.starts_with("GET /login.cgi "));
                ready_tx.send(()).unwrap();
                let _ = release_rx.recv_timeout(Duration::from_secs(6));
            });
            let token = std::sync::Arc::new(AtomicBool::new(false));
            let client_token = token.clone();
            let client = std::thread::spawn(move || {
                let target = Target {
                    url: String::new(),
                    host: "localhost".into(),
                    port,
                    secure: false,
                };
                request_path_inner(
                    &target,
                    "127.0.0.1",
                    &client_token,
                    true,
                    "/login.cgi",
                    Some(Instant::now() + Duration::from_secs(if cancel { 5 } else { 2 })),
                )
            });
            ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            if cancel {
                token.store(true, Ordering::Release);
            }
            let result = client.join().unwrap().unwrap_err();
            release_tx.send(()).unwrap();
            server.join().unwrap();
            assert_eq!(result.code, if cancel { "cancelled" } else { "timeout" });
        }
    }
    #[test]
    fn menu_auth_failure_is_not_hidden_by_oversized_body_header() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::park_timeout(Duration::from_millis(5))
                    }
                    Err(e) => panic!("peer {e}"),
                }
            };
            let _ = synthetic_menu_request(&mut stream);
            let _=stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/xml\r\nContent-Length: 20000\r\nConnection: close\r\n\r\n");
        });
        let target = Target {
            url: String::new(),
            host: "localhost".into(),
            port,
            secure: false,
        };
        let result = request_path_inner(
            &target,
            "127.0.0.1",
            &AtomicBool::new(false),
            true,
            "/xml/ui_mifi.xml",
            Some(Instant::now() + Duration::from_secs(5)),
        );
        assert_eq!(result.unwrap().0.status, Some(401));
        server.join().unwrap();
    }

    #[test]
    fn menu_loopback_caps_truncation_cancel_and_timeout() {
        for mode in ["oversize", "truncated", "cancel", "timeout"] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            listener.set_nonblocking(true).unwrap();
            let (started, ready) = std::sync::mpsc::channel();
            let (release, finish) = std::sync::mpsc::channel();
            let server = std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(6);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((s, _)) => break s,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            std::thread::park_timeout(Duration::from_millis(5))
                        }
                        Err(e) => panic!("peer {e}"),
                    }
                };
                let _ = synthetic_menu_request(&mut stream);
                let response = if mode == "oversize" {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: 16385\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: 100\r\nConnection: close\r\n\r\n<synthetic/>"
                };
                let _ = stream.write_all(response.as_bytes());
                let _ = started.send(());
                if mode == "cancel" || mode == "timeout" {
                    let _ = finish.recv_timeout(Duration::from_secs(7));
                }
            });
            let token = std::sync::Arc::new(AtomicBool::new(false));
            let worker_token = token.clone();
            let worker = std::thread::spawn(move || {
                let target = Target {
                    url: String::new(),
                    host: "localhost".into(),
                    port,
                    secure: false,
                };
                request_path_inner(
                    &target,
                    "127.0.0.1",
                    &worker_token,
                    true,
                    "/xml/ui_mifi.xml",
                    Some(Instant::now() + Duration::from_secs(5)),
                )
            });
            ready.recv_timeout(Duration::from_secs(5)).unwrap();
            if mode == "cancel" {
                token.store(true, Ordering::Release);
            }
            let result = worker.join().unwrap();
            let error = result.unwrap_err();
            match mode {
                "oversize" => assert_eq!(error.code, "metadata_too_large"),
                "cancel" => assert_eq!(error.code, "cancelled"),
                "timeout" => assert_eq!(error.code, "timeout"),
                _ => assert!(matches!(
                    error.code.as_str(),
                    "truncated_metadata" | "unreachable" | "unsupported_management"
                )),
            }
            let _ = release.send(());
            server.join().unwrap();
        }
    }
}
