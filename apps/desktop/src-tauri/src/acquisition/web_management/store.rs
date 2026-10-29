use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{atomic::AtomicBool, Mutex},
};

#[derive(Default)]
pub struct WebEvidenceStore {
    reports: Mutex<BTreeMap<String, WebEvidenceReport>>,
    pub(super) dialog_open: AtomicBool,
}

impl WebEvidenceStore {
    pub fn get_or_empty(&self, device_id: &str) -> Result<WebEvidenceReport, WebEvidenceError> {
        let reports = self
            .reports
            .lock()
            .map_err(|_| sanitize::fail("EVIDENCE_UNAVAILABLE", "Evidence store unavailable."))?;
        reports
            .get(device_id)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| empty_report(device_id))
    }

    pub fn import_har(
        &self,
        device_id: &str,
        bytes: &[u8],
    ) -> Result<WebEvidenceReport, WebEvidenceError> {
        self.merge(parse_har(device_id, bytes)?)
    }

    pub fn analyze_local_bundle(
        &self,
        device_id: &str,
        origin: &str,
        resource_name: &str,
        bytes: &[u8],
    ) -> Result<WebEvidenceReport, WebEvidenceError> {
        self.merge(analyze_bundle(device_id, origin, resource_name, bytes)?)
    }

    pub(crate) fn merge(
        &self,
        incoming: WebEvidenceReport,
    ) -> Result<WebEvidenceReport, WebEvidenceError> {
        let mut reports = self
            .reports
            .lock()
            .map_err(|_| sanitize::fail("EVIDENCE_UNAVAILABLE", "Evidence store unavailable."))?;
        if reports.len() >= 16 && !reports.contains_key(&incoming.device_id) {
            return Err(sanitize::fail("EVIDENCE_LIMIT", "At most 16 devices retain local capture evidence per application session. Restart to clear it."));
        }
        // Publication is transactional: a cap/error must preserve the preceding
        // sanitized report, including status and cross-reference state.
        let mut working = match reports.get(&incoming.device_id) {
            Some(report) => report.clone(),
            None => empty_report(&incoming.device_id)?,
        };
        let report = &mut working;
        for mut candidate in incoming.entries {
            // Both means two observation mechanisms, never independent trust.
            let matches = |entry: &EndpointEvidence| {
                entry.origin == candidate.origin && entry.path == candidate.path
            };
            if candidate.evidence == EvidenceSource::StaticCandidate {
                let mut observed = false;
                for entry in report
                    .entries
                    .iter_mut()
                    .filter(|e| matches(e) && e.evidence != EvidenceSource::StaticCandidate)
                {
                    entry.evidence = EvidenceSource::Both;
                    observed = true;
                }
                if observed {
                    continue;
                }
            } else {
                let static_seen = report.entries.iter().any(|e| {
                    matches(e)
                        && matches!(
                            e.evidence,
                            EvidenceSource::StaticCandidate | EvidenceSource::Both
                        )
                });
                if static_seen {
                    candidate.evidence = EvidenceSource::Both;
                }
                report
                    .entries
                    .retain(|e| !(matches(e) && e.evidence == EvidenceSource::StaticCandidate));
            }
            if let Some(existing) = report.entries.iter_mut().find(|e| e.id == candidate.id) {
                if existing.evidence == EvidenceSource::Both {
                    candidate.evidence = EvidenceSource::Both;
                }
                candidate
                    .query_names
                    .extend(existing.query_names.iter().cloned());
                candidate
                    .request_body_names
                    .extend(existing.request_body_names.iter().cloned());
                *existing = candidate;
            } else if report.entries.len() < 512 {
                report.entries.push(candidate);
            } else {
                return Err(sanitize::fail(
                    "EVIDENCE_LIMIT",
                    "Combined device evidence limit is 512 endpoints.",
                ));
            }
        }
        report.auth_candidates.extend(incoming.auth_candidates);
        report.resources.extend(incoming.resources);
        report.warnings.extend(incoming.warnings);
        finish(report);
        let result = report.clone();
        reports.insert(working.device_id.clone(), working);
        Ok(result)
    }
}

pub(super) fn finish(report: &mut WebEvidenceReport) {
    let mut unique: BTreeMap<String, EndpointEvidence> = BTreeMap::new();
    for mut entry in std::mem::take(&mut report.entries) {
        if let Some(previous) = unique.remove(&entry.id) {
            entry.query_names.extend(previous.query_names);
            entry.request_body_names.extend(previous.request_body_names);
            if previous.evidence == EvidenceSource::Both {
                entry.evidence = EvidenceSource::Both;
            }
        }
        unique.insert(entry.id.clone(), entry);
    }
    report.entries = unique.into_values().collect();
    report.entries.sort_by(|a, b| {
        (&a.origin, &a.path, &a.method, &a.id).cmp(&(&b.origin, &b.path, &b.method, &b.id))
    });
    report.auth_candidates.sort();
    report.auth_candidates.dedup();
    report
        .resources
        .sort_by(|a, b| (&a.origin, &a.path, &a.kind).cmp(&(&b.origin, &b.path, &b.kind)));
    report
        .resources
        .dedup_by(|a, b| a.origin == b.origin && a.path == b.path && a.kind == b.kind);
    if report.resources.len() > 64 {
        report.warnings.push(
            "Combined resource summary reached its 64-reference cap; evidence is partial.".into(),
        );
    }
    report.resources.truncate(64);
    report.warnings.sort();
    report.warnings.dedup();
    report.warnings.truncate(8);
    report.counts.clear();
    for entry in &report.entries {
        let category = serde_json::to_value(&entry.classification)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".into());
        *report.counts.entry(category).or_default() += 1;
    }
    // Hard gates are not influenced by imported evidence or source labels.
    report.can_acquire = false;
    report.actions.authenticate = false;
    report.actions.collect_resources = false;
    for entry in &mut report.entries {
        entry.read_only_qualified = false;
    }
    // BTreeSet produces stable capped schema-name collections.
    for entry in &mut report.entries {
        entry.query_names = entry
            .query_names
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(64)
            .collect();
        entry.request_body_names = entry
            .request_body_names
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(64)
            .collect();
    }
    report.strategy = strategy::WebManagementAcquisition::assess(report);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn capture(url: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({"log":{"entries":[{"request":{"method":"GET","url":url},"response":{"status":200}}]}})).unwrap()
    }

    #[test]
    fn synthetic_cross_reference_requires_same_origin_and_never_grants_trust() {
        let store = WebEvidenceStore::default();
        store
            .import_har("d", &capture("http://192.0.2.1/api/status"))
            .unwrap();
        let report = store
            .analyze_local_bundle(
                "d",
                "http://192.0.2.1",
                "app.js",
                br#"fetch('/api/status')"#,
            )
            .unwrap();
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].evidence, EvidenceSource::Both);
        assert!(!report.entries[0].read_only_qualified);
        let report = store
            .analyze_local_bundle(
                "d",
                "http://192.0.2.2",
                "app.js",
                br#"fetch('/api/status')"#,
            )
            .unwrap();
        assert_eq!(report.entries.len(), 2);
        assert!(report.entries.iter().any(
            |e| e.origin == "http://192.0.2.2" && e.evidence == EvidenceSource::StaticCandidate
        ));
        assert!(!report.can_acquire);
    }

    #[test]
    fn synthetic_static_first_import_and_repeated_captures_stay_stable() {
        let store = WebEvidenceStore::default();
        store
            .analyze_local_bundle(
                "d",
                "http://192.0.2.1",
                "app.js",
                br#"fetch('/api/status')"#,
            )
            .unwrap();
        let first = store
            .import_har(
                "d",
                &capture("http://192.0.2.1/api/status?token=secret-one"),
            )
            .unwrap();
        let second = store
            .import_har(
                "d",
                &capture("http://192.0.2.1/api/status?token=secret-two"),
            )
            .unwrap();
        assert_eq!(first.entries.len(), 1);
        assert_eq!(first.entries[0].evidence, EvidenceSource::Both);
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert!(WebEvidenceStore::default()
            .get_or_empty("d")
            .unwrap()
            .entries
            .is_empty());
    }

    #[test]
    fn synthetic_device_labels_never_change_source_assessment() {
        let first = parse_har(
            "router-label-one",
            &capture("http://192.0.2.1/firmware/download"),
        )
        .unwrap();
        let second = parse_har(
            "entirely-different-label",
            &capture("http://192.0.2.1/firmware/download"),
        )
        .unwrap();
        assert_eq!(
            strategy::WebManagementAcquisition::assess(&first),
            strategy::WebManagementAcquisition::assess(&second)
        );
    }

    #[test]
    fn synthetic_memory_device_limit_is_bounded_and_local_actions_do_not_authenticate() {
        let store = WebEvidenceStore::default();
        for index in 0..16 {
            store
                .import_har(&format!("d{index}"), &capture("http://192.0.2.1/status"))
                .unwrap();
        }
        assert_eq!(
            store
                .import_har("extra", &capture("http://192.0.2.1/status"))
                .unwrap_err()
                .code,
            "EVIDENCE_LIMIT"
        );
        let report = store.get_or_empty("unseen").unwrap();
        assert!(report.actions.import_har && report.actions.analyze_local_bundle);
        assert!(!report.actions.collect_resources && !report.actions.authenticate);
    }

    #[test]
    fn synthetic_rejected_merge_keeps_previous_report_unchanged() {
        let store = WebEvidenceStore::default();
        let requests: Vec<_> = (0..512).map(|n| json!({"request":{"method":"GET","url":format!("http://192.0.2.1/api/{n}/status")},"response":{"status":200}})).collect();
        let bytes = serde_json::to_vec(&json!({"log":{"entries":requests}})).unwrap();
        let before = store.import_har("d", &bytes).unwrap();
        let incoming = serde_json::to_vec(&json!({"log":{"entries":[
            {"request":{"method":"GET","url":"http://192.0.2.1/api/0/status"},"response":{"status":401}},
            {"request":{"method":"GET","url":"http://192.0.2.1/new/status"},"response":{"status":200}}
        ]}})).unwrap();
        assert_eq!(
            store.import_har("d", &incoming).unwrap_err().code,
            "EVIDENCE_LIMIT"
        );
        assert_eq!(
            serde_json::to_string(&before).unwrap(),
            serde_json::to_string(&store.get_or_empty("d").unwrap()).unwrap()
        );
    }
}
