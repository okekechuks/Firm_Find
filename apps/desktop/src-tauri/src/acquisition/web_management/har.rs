use super::{sanitize::*, *};
use serde_json::Value;
use std::collections::BTreeSet;

const MAX_ENTRIES: usize = 512;

pub(super) fn parse(device_id: &str, bytes: &[u8]) -> Result<WebEvidenceReport, WebEvidenceError> {
    if bytes.len() > MAX_HAR_BYTES {
        return Err(fail(
            "INPUT_TOO_LARGE",
            "HAR limit is 8 MiB. Export a smaller sanitized capture.",
        ));
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| {
        fail(
            "INVALID_HAR",
            "Selected file is not a bounded valid HAR JSON document.",
        )
    })?;
    let entries = value
        .get("log")
        .and_then(|v| v.get("entries"))
        .and_then(Value::as_array)
        .ok_or_else(|| fail("INVALID_HAR", "HAR must contain log.entries."))?;
    if entries.len() > MAX_ENTRIES {
        return Err(fail(
            "TOO_MANY_ENTRIES",
            "HAR limit is 512 requests. Export a smaller capture.",
        ));
    }
    let mut report = empty_report(device_id)?;
    let mut skipped = 0;
    for entry in entries {
        let Some(request) = entry.get("request") else {
            skipped += 1;
            continue;
        };
        let Some(raw) = request.get("url").and_then(Value::as_str) else {
            skipped += 1;
            continue;
        };
        let Some(url) = http_url(raw) else {
            skipped += 1;
            continue;
        };
        let method = method(
            request
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN"),
        );
        let mut operation_url = url.clone();
        let mut query_names = query_names(&url).into_iter().collect::<BTreeSet<_>>();
        if let Some(query) = request.get("queryString").and_then(Value::as_array) {
            for pair in query.iter().take(128) {
                let Some(key) = pair.get("name").and_then(Value::as_str) else {
                    continue;
                };
                query_names.insert(name(key));
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
                    if let Some(value) = pair
                        .get("value")
                        .and_then(Value::as_str)
                        .filter(|v| v.len() <= 128)
                    {
                        operation_url.query_pairs_mut().append_pair(key, value);
                    }
                }
            }
        }
        let classification = classify(&method, &operation_url);
        let response = entry.get("response").unwrap_or(&Value::Null);
        let mut selected_headers = BTreeMap::new();
        header_metadata(request, &mut selected_headers, &mut report.auth_candidates);
        header_metadata(response, &mut selected_headers, &mut report.auth_candidates);
        let body = request.get("postData").unwrap_or(&Value::Null);
        let body_names = body_names(body);
        if classification == Classification::Authentication && !body_names.is_empty() {
            report.auth_candidates.push("form_session".into());
        }
        let mime_type = response
            .get("content")
            .and_then(|v| v.get("mimeType"))
            .and_then(Value::as_str)
            .map(mime);
        let request_content_type = body
            .get("mimeType")
            .and_then(Value::as_str)
            .map(mime)
            .or_else(|| request_headers_mime(request));
        let context = entry
            .get("_initiator")
            .and_then(|v| v.get("type"))
            .and_then(Value::as_str)
            .filter(|v| ["script", "parser", "preload", "preflight", "other"].contains(v))
            .map(str::to_owned);
        let origin = display_origin(&url);
        let path = path(&url);
        let id = evidence_id(&origin, &path, &method, &classification);
        report.entries.push(EndpointEvidence {
            id,
            origin,
            path,
            method,
            query_names: query_names.into_iter().take(64).collect(),
            classification,
            evidence: EvidenceSource::HarObserved,
            read_only_qualified: false,
            status: response
                .get("status")
                .and_then(Value::as_u64)
                .filter(|v| (100..=599).contains(v))
                .map(|v| v as u16),
            mime_type,
            request_content_type,
            request_body_names: body_names,
            selected_headers,
            context,
            response_size: response
                .get("content")
                .and_then(|v| v.get("size"))
                .and_then(Value::as_u64)
                .filter(|v| *v <= 2 * 1024 * 1024 * 1024),
        });
    }
    if skipped != 0 {
        report.warnings.push(format!(
            "{skipped} malformed or unsupported HAR request entries were omitted."
        ));
    }
    report.warnings.push("HAR contains only observed browser requests; absence of an endpoint proves nothing. Imported evidence never authorizes replay.".into());
    finish(&mut report);
    Ok(report)
}

fn request_headers_mime(request: &Value) -> Option<String> {
    request
        .get("headers")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|h| {
            h.get("name")
                .and_then(Value::as_str)
                .filter(|n| n.eq_ignore_ascii_case("content-type"))?;
            h.get("value").and_then(Value::as_str).map(mime)
        })
}

fn header_metadata(item: &Value, selected: &mut BTreeMap<String, String>, auth: &mut Vec<String>) {
    let Some(headers) = item.get("headers").and_then(Value::as_array) else {
        return;
    };
    for header in headers.iter().take(256) {
        let Some(name) = header.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(value) = header.get("value").and_then(Value::as_str) else {
            continue;
        };
        match name.to_ascii_lowercase().as_str() {
            "content-type" => {
                selected.insert("content-type".into(), mime(value));
            }
            "content-length" => {
                if let Ok(size) = value.trim().parse::<u64>() {
                    if size <= 2 * 1024 * 1024 * 1024 {
                        selected.insert("content-length".into(), size.to_string());
                    }
                }
            }
            "allow" => {
                let methods = value
                    .split(',')
                    .map(str::trim)
                    .filter(|v| {
                        ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"].contains(v)
                    })
                    .collect::<BTreeSet<_>>();
                if !methods.is_empty() {
                    selected.insert(
                        "allow".into(),
                        methods.into_iter().collect::<Vec<_>>().join(", "),
                    );
                }
            }
            "www-authenticate" | "authorization" => {
                let value = value.to_ascii_lowercase();
                // Persist fixed scheme names only, never challenge or credential material.
                if value.starts_with("digest ") {
                    auth.push("digest".into());
                } else if value.starts_with("basic ") {
                    auth.push("basic".into());
                } else {
                    auth.push("unknown".into());
                }
            }
            "cookie" | "set-cookie" => auth.push("cookie_session".into()),
            _ => {}
        }
    }
}

fn body_names(body: &Value) -> Vec<String> {
    let mut names = BTreeSet::new();
    if let Some(params) = body.get("params").and_then(Value::as_array) {
        names.extend(
            params
                .iter()
                .take(128)
                .filter_map(|p| p.get("name").and_then(Value::as_str))
                .map(name),
        );
    }
    let raw_mime = body.get("mimeType").and_then(Value::as_str).unwrap_or("");
    if let Some(text) = body
        .get("text")
        .and_then(Value::as_str)
        .filter(|v| v.len() <= 64 * 1024)
    {
        if mime(raw_mime) == "application/json" {
            if let Ok(Value::Object(object)) = serde_json::from_str::<Value>(text) {
                names.extend(object.keys().take(128).map(|v| name(v)));
            }
        } else if mime(raw_mime) == "application/x-www-form-urlencoded" {
            names.extend(
                url::form_urlencoded::parse(text.as_bytes())
                    .take(128)
                    .map(|(key, _)| name(&key)),
            );
        }
    }
    names.into_iter().take(64).collect()
}
