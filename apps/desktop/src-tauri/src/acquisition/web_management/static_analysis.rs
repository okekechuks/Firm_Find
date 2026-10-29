use super::{sanitize::*, *};
use std::collections::BTreeSet;

/// A lexical scan only. No parser plugin, DOM, script engine or command runner.
pub(super) fn parse(
    device_id: &str,
    raw_origin: &str,
    resource_name: &str,
    bytes: &[u8],
) -> Result<WebEvidenceReport, WebEvidenceError> {
    let base = origin(raw_origin)?;
    if bytes.len() > MAX_BUNDLE_BYTES {
        return Err(fail(
            "INPUT_TOO_LARGE",
            "Management resource limit is 1 MiB.",
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| {
        fail(
            "INVALID_TEXT",
            "Select UTF-8 HTML, JavaScript, JSON or CSS data.",
        )
    })?;
    let resource = base.join(resource_name).map_err(|_| {
        fail(
            "UNSUPPORTED_RESOURCE",
            "Select an HTML, JS, JSON or CSS resource.",
        )
    })?;
    let kind = resource_kind(&resource).ok_or_else(|| {
        fail(
            "UNSUPPORTED_RESOURCE",
            "Select an HTML, JS, JSON or CSS resource.",
        )
    })?;
    let mut report = empty_report(device_id)?;
    if kind == "html" {
        let attributes = super::super::resource_attributes(text);
        if attributes.len() > 128 {
            report
                .warnings
                .push("HTML reference scan reached its cap; resource evidence is partial.".into());
        }
        for reference in attributes.into_iter().take(128) {
            let Ok(url) = base.join(reference) else {
                continue;
            };
            if url.origin() != base.origin()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                continue;
            }
            let Some(kind) = resource_kind(&url) else {
                continue;
            };
            report.resources.push(ResourceEvidence {
                origin: display_origin(&url),
                path: path(&url),
                kind: kind.into(),
                referenced: true,
            });
        }
        if report.resources.len() > 64 {
            report.warnings.push(
                "Resource summary reached its 64-reference cap; resource evidence is partial."
                    .into(),
            );
        }
    }
    let mut seen = BTreeSet::new();
    let literals = string_literals(text);
    if literals.len() >= 4096 {
        report.warnings.push(
            "String scan reached its 4096-candidate cap; static evidence may be partial.".into(),
        );
    }
    for (literal, location) in literals {
        if !route_literal(&literal) {
            continue;
        }
        let Ok(url) = base.join(&literal) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            continue;
        }
        let classification = classify("UNKNOWN", &url);
        let origin = display_origin(&url);
        let path = path(&url);
        let id = evidence_id(&origin, &path, "UNKNOWN", &classification);
        if !seen.insert(id.clone()) {
            continue;
        }
        report.entries.push(EndpointEvidence {
            id,
            origin,
            path,
            method: "UNKNOWN".into(),
            query_names: query_names(&url),
            classification,
            evidence: EvidenceSource::StaticCandidate,
            read_only_qualified: false,
            status: None,
            mime_type: None,
            request_content_type: None,
            request_body_names: vec![],
            selected_headers: BTreeMap::new(),
            context: Some(wrapper_context(text, location).into()),
            response_size: None,
        });
        if report.entries.len() >= 512 {
            report.warnings.push(
                "Endpoint summary reached its 512-candidate cap; static evidence may be partial."
                    .into(),
            );
            break;
        }
    }
    report.warnings.push("Static candidates are strings, not verified routes. Dynamic construction may be missed. No scripts execute and no candidate is replayed.".into());
    report.warnings.push("Local bundle origin is user-selected context, not proof that bytes came from that device. Live collection remains disabled until separately qualified and approved.".into());
    finish(&mut report);
    Ok(report)
}

fn route_literal(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 2048
        || value.contains("${")
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return false;
    }
    value.starts_with('/')
        || value.starts_with("http://")
        || value.starts_with("https://")
        || [".cgi", ".php", ".json", ".xml"]
            .iter()
            .any(|ext| value.split('?').next().is_some_and(|v| v.ends_with(ext)))
}

fn wrapper_context(text: &str, location: usize) -> &'static str {
    let prefix = String::from_utf8_lossy(&text.as_bytes()[location.saturating_sub(96)..location]);
    match (prefix.rfind("fetch("), prefix.rfind(".open(")) {
        (Some(fetch), Some(xhr)) if xhr > fetch => "xmlhttprequest_wrapper",
        (Some(_), _) => "fetch_wrapper",
        (_, Some(_)) => "xmlhttprequest_wrapper",
        _ => "string_literal",
    }
}

fn string_literals(text: &str) -> Vec<(String, usize)> {
    let bytes = text.as_bytes();
    let mut at = 0;
    let mut result = vec![];
    while at < bytes.len() && result.len() < 4096 {
        // Ignore comment text; comments do not construct routes.
        if bytes[at..].starts_with(b"//") {
            at += 2;
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
            continue;
        }
        if bytes[at..].starts_with(b"/*") {
            at += 2;
            while at < bytes.len() && !bytes[at..].starts_with(b"*/") {
                at += 1;
            }
            at = (at + 2).min(bytes.len());
            continue;
        }
        let quote = bytes[at];
        if !b"'\"`".contains(&quote) {
            at += 1;
            continue;
        }
        let location = at;
        at += 1;
        let mut value = Vec::new();
        let mut closed = false;
        while at < bytes.len() {
            let c = bytes[at];
            at += 1;
            if c == quote {
                closed = true;
                break;
            }
            if c == b'\\' && at < bytes.len() {
                let escaped = bytes[at];
                at += 1;
                if b"/\\'\"`".contains(&escaped) {
                    value.push(escaped);
                } else {
                    value.push(b'?');
                }
            } else {
                value.push(c);
            }
            if value.len() > 2048 {
                while at < bytes.len() && bytes[at] != quote {
                    at += 1;
                }
                at = (at + 1).min(bytes.len());
                break;
            }
        }
        if closed {
            if let Ok(value) = String::from_utf8(value) {
                result.push((value, location));
            }
        }
    }
    result
}
