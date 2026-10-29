//! Intentionally lossy display normalization. Output is not an executable URI.
use super::{Classification, WebEvidenceError};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use url::Url;

pub(super) fn fail(code: &'static str, message: &'static str) -> WebEvidenceError {
    WebEvidenceError { code, message }
}

pub(super) fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub(super) fn origin(raw: &str) -> Result<Url, WebEvidenceError> {
    let url = Url::parse(raw)
        .map_err(|_| fail("INVALID_ORIGIN", "Select an HTTP/HTTPS management origin."))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err(fail(
            "INVALID_ORIGIN",
            "Use an HTTP/HTTPS origin without credentials, path, query or fragment.",
        ));
    }
    Ok(url)
}

pub(super) fn http_url(raw: &str) -> Option<Url> {
    if raw.len() > 4096 || raw.chars().any(char::is_control) {
        return None;
    }
    let url = Url::parse(raw).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then_some(url)
}

pub(super) fn display_origin(url: &Url) -> String {
    match url.host() {
        Some(url::Host::Ipv4(_)) | Some(url::Host::Ipv6(_)) => url.origin().ascii_serialization(),
        _ => {
            // A DNS label can carry a session/user identifier. Preserve equality,
            // never that arbitrary label; this deliberately cannot be replayed.
            let host = format!(
                "[redacted-host:{}]",
                &digest(url.host_str().unwrap_or(""))[..12]
            );
            let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
            format!("{}://{host}{port}", url.scheme())
        }
    }
}

// Only protocol vocabulary is preserved. Arbitrary field/path names may themselves
// contain credentials or identifiers and are represented by non-replayable hashes.
const WORDS: &[&str] = &[
    "api",
    "cgi-bin",
    "cgi",
    "v1",
    "v2",
    "v3",
    "js",
    "css",
    "xml",
    "json",
    "assets",
    "static",
    "scripts",
    "index",
    "main",
    "app",
    "bundle",
    "dashboard",
    "device",
    "device_name",
    "model",
    "manufacturer",
    "hardware",
    "revision",
    "version",
    "version_num",
    "version_num.txt",
    "firmware",
    "fota",
    "software",
    "metadata",
    "manifest",
    "status",
    "status1",
    "sysinfo",
    "system",
    "info",
    "identity",
    "network",
    "configuration",
    "config",
    "settings",
    "get",
    "read",
    "list",
    "show",
    "set",
    "save",
    "write",
    "update",
    "check",
    "checkupdate",
    "upgrade",
    "upload",
    "download",
    "backup",
    "export",
    "import",
    "restore",
    "reset",
    "factory",
    "factoryreset",
    "reboot",
    "restart",
    "login",
    "logout",
    "auth",
    "authenticate",
    "authentication",
    "session",
    "digest",
    "basic",
    "action",
    "file",
    "filename",
    "type",
    "format",
    "mode",
    "id",
    "name",
    "username",
    "password",
    "token",
    "csrf",
    "csrf_token",
    "nonce",
    "realm",
    "qop",
    "algorithm",
    "imei",
    "imsi",
    "iccid",
    "sim",
    "serial",
    "xml_action",
    "ui_mifi",
    "true",
    "false",
    "operation",
    "command",
    "cmd",
    "method",
    "content",
    "data",
];

fn known(value: &str) -> bool {
    WORDS.contains(&value.to_ascii_lowercase().as_str())
}

pub(super) fn name(value: &str) -> String {
    if value.len() <= 48 && known(value) {
        value.to_ascii_lowercase()
    } else {
        format!("field-{}", &digest(value)[..12])
    }
}

pub(super) fn path(url: &Url) -> String {
    let mut out = String::new();
    for segment in url.path().split('/').skip(1).take(24) {
        out.push('/');
        if segment.is_empty() {
            continue;
        }
        let lower = segment.to_ascii_lowercase();
        if lower.len() <= 48 && known(&lower) {
            out.push_str(&lower);
            continue;
        }
        let (base, extension) = lower.rsplit_once('.').unwrap_or((&lower, ""));
        let safe_extension = [
            "cgi", "php", "asp", "aspx", "json", "xml", "js", "mjs", "css", "html", "htm", "txt",
            "bin", "img", "fw",
        ]
        .contains(&extension);
        if safe_extension && known(base) {
            out.push_str(base);
            out.push('.');
            out.push_str(extension);
        } else {
            out.push_str("opaque-");
            out.push_str(&digest(segment)[..12]);
            if safe_extension {
                out.push('.');
                out.push_str(extension);
            }
        }
    }
    if out.is_empty() {
        out.push('/');
    }
    out
}

pub(super) fn query_names(url: &Url) -> Vec<String> {
    url.query_pairs()
        .map(|(key, _)| name(&key))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(64)
        .collect()
}

pub(super) fn mime(raw: &str) -> String {
    let base = raw
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match base.as_str() {
        "text/html"
        | "text/plain"
        | "text/css"
        | "text/javascript"
        | "application/javascript"
        | "application/json"
        | "application/xml"
        | "text/xml"
        | "application/octet-stream"
        | "application/x-www-form-urlencoded"
        | "multipart/form-data"
        | "application/zip"
        | "application/gzip"
        | "application/x-tar" => base,
        "" => "unknown".into(),
        _ => "other".into(),
    }
}

pub(super) fn method(raw: &str) -> String {
    match raw {
        "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS" => raw.into(),
        _ => "UNKNOWN".into(),
    }
}

fn words(raw: &str) -> Vec<String> {
    let mut expanded = String::with_capacity(raw.len());
    let mut lower_before = false;
    for c in raw.chars() {
        if c.is_ascii_uppercase() && lower_before {
            expanded.push(' ');
        }
        expanded.push(c);
        lower_before = c.is_ascii_lowercase();
    }
    expanded
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|v| !v.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

pub(super) fn classify(method: &str, url: &Url) -> Classification {
    // Query values are used transiently only to recognize dangerous operations.
    // They are never emitted, logged, cached or used as request authority.
    let mut tokens = words(url.path());
    for (key, value) in url.query_pairs() {
        if [
            "action",
            "act",
            "operation",
            "op",
            "cmd",
            "command",
            "method",
        ]
        .contains(&key.to_ascii_lowercase().as_str())
        {
            tokens.extend(words(&value));
        }
    }
    let has = |list: &[&str]| tokens.iter().any(|v| list.contains(&v.as_str()));
    let config = has(&["config", "configuration", "settings"]);
    let firmware = has(&["firmware", "fota", "software", "image"]);
    // Operation risk always wins over a benign status/version token.
    if has(&["factoryreset", "reset", "factory"]) {
        return Classification::FactoryReset;
    }
    if has(&["reboot", "restart"]) {
        return Classification::Reboot;
    }
    if config && has(&["restore", "import"]) {
        return Classification::ConfigRestore;
    }
    if firmware && has(&["upload", "upgrade", "flash"]) {
        return Classification::FirmwareUpload;
    }
    if has(&[
        "login",
        "logout",
        "auth",
        "authenticate",
        "authentication",
        "session",
        "digest",
    ]) {
        return Classification::Authentication;
    }
    if config && has(&["backup", "export", "download"]) {
        return Classification::ConfigBackup;
    }
    if firmware && has(&["download"]) {
        return Classification::FirmwareDownload;
    }
    if firmware && has(&["check", "checkupdate"]) {
        return Classification::FirmwareUpdateCheck;
    }
    if config
        && (has(&["set", "save", "write", "update", "upload"]) || !matches!(method, "GET" | "HEAD"))
    {
        return Classification::ConfigWrite;
    }
    if has(&[
        "set", "save", "write", "update", "upload", "delete", "format", "exec", "execute", "debug",
    ]) {
        return Classification::Unknown;
    }
    if !matches!(method, "GET" | "HEAD" | "UNKNOWN") {
        return Classification::Unknown;
    }
    if firmware || has(&["manifest"]) {
        return Classification::FirmwareMetadata;
    }
    if config {
        return Classification::ConfigRead;
    }
    if has(&["export", "download", "backup"]) {
        return Classification::FileExport;
    }
    if has(&[
        "identity",
        "sysinfo",
        "device",
        "deviceinfo",
        "model",
        "version",
        "version_num",
    ]) {
        return Classification::IdentityRead;
    }
    if has(&["status", "status1", "network"]) {
        return Classification::StatusRead;
    }
    Classification::Unknown
}

pub(super) fn resource_kind(url: &Url) -> Option<&'static str> {
    let path = url.path().to_ascii_lowercase();
    if path.ends_with(".js") || path.ends_with(".mjs") {
        Some("javascript")
    } else if path.ends_with(".html") || path.ends_with(".htm") {
        Some("html")
    } else if path.ends_with(".json") {
        Some("json")
    } else if path.ends_with(".css") {
        Some("css")
    } else {
        None
    }
}
