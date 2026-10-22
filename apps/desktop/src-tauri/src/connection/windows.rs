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
        // A single TCP preflight distinguishes refusal from HTTP/TLS transport errors.
        match TcpStream::connect_timeout(&socket, Duration::from_millis(800)) {
            Ok(stream) => drop(stream),
            Err(e) => return Err(EndpointResult::failure(match e.kind() { std::io::ErrorKind::ConnectionRefused => "connection_refused", std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => "timeout", _ => "unreachable" }, "Router management port did not accept the bounded connection. Check address, port and connection.")),
        }
        cancelled(token)?;
        let pinned = if matches!(ip, IpAddr::V6(v) if v.is_unicast_link_local()) {
            format!("{ip}%{index}")
        } else {
            ip.to_string()
        };
        let (mut result, body) = request(target, &pinned, token, root)?;
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

fn request(
    target: &Target,
    pinned: &str,
    token: &AtomicBool,
    root: bool,
) -> Result<(EndpointResult, Option<Vec<u8>>), Box<EndpointResult>> {
    cancelled(token)?;
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
            wide("/").as_ptr(),
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
    set(&request, WINHTTP_OPTION_MAX_RESPONSE_HEADER_SIZE, 16384)?;
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
    let deadline = Instant::now() + Duration::from_secs(3);
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
    let mut body = None;
    if root && (200..300).contains(&status) {
        let mut bytes = Vec::new();
        while bytes.len() < 16384 {
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
            bytes.extend_from_slice(&chunk[..chunk.len().min(16384 - bytes.len())]);
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
}
