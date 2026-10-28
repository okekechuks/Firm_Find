//! Independent validation of bounded, version-2 inspection metadata.
//! File bytes are independently hashed/indexed by the parent module.
use super::*;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Layer {
    pub id: String,
    pub parent_id: Option<String>,
    pub space: String,
    pub offset: u64,
    pub length: u64,
    pub source_path: String,
    pub input_sha256: String,
    pub handler: String,
    pub handler_version: u32,
    pub format: String,
    pub confidence: String,
    pub decoded_bytes: u64,
    pub completeness: String,
    pub reasons: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Unexplained {
    pub layer_id: String,
    pub offset: u64,
    pub length: u64,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entropy {
    pub offset: u64,
    pub length: u64,
    pub sample_bytes: u64,
    pub bits_per_byte: f64,
    pub classification: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scan {
    pub bytes: u64,
    pub all_offsets_scanned: bool,
    pub candidate_limit: u64,
    pub candidates_retained: u64,
    pub candidates_capped: bool,
    pub entropy_sampled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tool {
    pub name: String,
    pub version: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub engine_version: String,
    pub engine_sha256: String,
    pub python_version: String,
    pub tools: Vec<Tool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Budgets {
    pub max_layer_depth: u64,
    pub max_layers: u64,
    pub expanded_bytes: u64,
    pub max_expanded_bytes: u64,
    pub max_file_bytes: u64,
    pub ratio_limit: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwDeclaration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_requirements: Option<BTreeMap<String, String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Swupdate {
    pub layer_id: String,
    pub versions: Vec<String>,
    pub declarations: Vec<SwDeclaration>,
    pub authority: String,
    pub signature_verified: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmittedMember {
    pub layer_id: String,
    pub path: String,
    pub declared_bytes: u64,
    pub reason: String,
}
fn verdict(value: &str) -> bool {
    matches!(
        value,
        "complete" | "partial" | "unsupported" | "encrypted-or-unknown"
    )
}
fn bounded_text(value: &str, limit: usize) -> bool {
    value.len() <= limit && !value.chars().any(|c| c == '\0')
}
fn range(offset: u64, length: u64, limit: u64) -> bool {
    length > 0 && offset.checked_add(length).is_some_and(|end| end <= limit)
}
pub(super) fn validate(report: &ExtractionReport, hash: &str, size: u64) -> Result<()> {
    let completeness = report.completeness.as_deref().ok_or_else(invalid)?;
    let layers = report.layers.as_ref().ok_or_else(invalid)?;
    let gaps = report.unexplained_regions.as_ref().ok_or_else(invalid)?;
    let entropy = report.entropy_map.as_ref().ok_or_else(invalid)?;
    let provenance = report.provenance.as_ref().ok_or_else(invalid)?;
    let budgets = report.budgets.as_ref().ok_or_else(invalid)?;
    let scan = report.scan.as_ref().ok_or_else(invalid)?;
    let swupdate = report.swupdate.as_ref().ok_or_else(invalid)?;
    let omitted = report.omitted_members.as_ref().ok_or_else(invalid)?;
    let engine_hash = format!("{:x}", Sha256::digest(ENGINE.as_bytes()));
    if report.input_sha256 != hash
        || report.input_bytes != size
        || size == 0
        || size > MAX_INPUT
        || report.worker_exit != 0
        || report.created_at != 0
        || !verdict(completeness)
        || report.extractor != "OBTAINER layered engine 2"
        || report.tool_sha256 != engine_hash
        || provenance.engine_sha256 != engine_hash
        || provenance.engine_version != "2.0.0"
        || provenance.python_version != "3.14.4"
        || provenance.tools.len() > 16
        || report.isolation != expected_isolation()
        || report.entries.len() > 20000
        || report.total_bytes > MAX_OUTPUT
        || report.expanded_bytes > MAX_INPUT
        || !bounded_text(&report.container, 40)
        || !bounded_text(&report.region.format, 40)
        || !range(report.region.offset, report.region.length, MAX_INPUT)
        || report.region.compression > 6
        || layers.is_empty()
        || layers.len() > 256
        || gaps.len() > 4096
        || entropy.len() > 512
        || swupdate.len() > 256
        || omitted.len() > 4096
        || budgets.max_layer_depth != 6
        || budgets.max_layers != 256
        || budgets.max_expanded_bytes != MAX_OUTPUT
        || budgets.max_file_bytes != 32 * 1024 * 1024
        || budgets.ratio_limit != 1024
        || budgets.expanded_bytes > MAX_OUTPUT
        || report.expanded_bytes
            != if budgets.expanded_bytes > 0 {
                budgets.expanded_bytes
            } else {
                size
            }
        || scan.bytes != size
        || !scan.all_offsets_scanned
        || !scan.entropy_sampled
        || scan.candidate_limit != 4096
        || scan.candidates_retained > 4096
        || scan.candidates_capped != (scan.candidates_retained == 4096)
    {
        return Err(invalid());
    }
    let pins = [
        (
            "/usr/bin/python3.14",
            "fa9796cd3a30878e11a2f40372f773d3fcd913fff35e5bee8dd9a036e22e93ab",
        ),
        (
            "/usr/bin/unsquashfs",
            "003a7e9d70fbdcd05dfe132101c4b7bbb19a217976777f5393573eff21ddc202",
        ),
        (
            "/otrx",
            "66b870899eca6a12079668c23cdd95db069978afc9af5bcbc3681cd9c1e968e5",
        ),
        (
            "/usr/bin/zstd",
            "fe50fd600cad89b775dc5b4b10bc8ff95e2d33ddb4f7632966464b1fbdab8598",
        ),
        (
            "/usr/bin/lz4",
            "4be960d6f6b0d7ef69e01a9e1a056591c17b8687e9851db128018b2ac5f01da0",
        ),
        (
            "/usr/sbin/debugfs",
            "864e1d7b445e7b5bfc831da78330dbcafc590fa82b89ea9de60b7527f989954f",
        ),
        (
            "/usr/sbin/e2fsck",
            "2e51f521c676729920eaba694933d9d4048645f1a5789556fd0027e62d11ecc8",
        ),
        (
            "/usr/sbin/fsck.cramfs",
            "82ccbc99540cc47c751cfb1fe4a0d77ebc97ffbc1324a2b5effb19f84970869e",
        ),
        (
            "/usr/lib/x86_64-linux-gnu/libarchive.so.13.8.5",
            "959dcce452c258ad7016f74b33b6df688cdad8a6caa1a625b1bfbebe85b332b0",
        ),
    ];
    let mut tools = HashSet::new();
    for tool in &provenance.tools {
        if !tools.insert(&tool.name)
            || !bounded_text(&tool.version, 128)
            || !pins
                .iter()
                .any(|(name, sha)| tool.name == *name && tool.sha256 == *sha)
        {
            return Err(invalid());
        }
    }
    let mut index = HashMap::new();
    let mut depths = HashMap::new();
    for (number, layer) in layers.iter().enumerate() {
        if layer.id != format!("layer-{number:04}")
            || !valid_hash(&layer.input_sha256)
            || !verdict(&layer.completeness)
            || !matches!(layer.space.as_str(), "source" | "expanded" | "member")
            || !matches!(layer.confidence.as_str(), "signature" | "validated")
            || layer.handler_version != 1
            || !bounded_text(&layer.handler, 40)
            || !bounded_text(&layer.format, 40)
            || !bounded_text(&layer.source_path, 240)
            || layer.decoded_bytes > MAX_OUTPUT
            || layer.length == 0
            || layer.reasons.len() > 4096
            || layer
                .reasons
                .iter()
                .any(|reason| !bounded_text(reason, 1024))
        {
            return Err(invalid());
        }
        let depth = if let Some(parent) = layer.parent_id.as_deref() {
            let parent: &&Layer = index.get(parent).ok_or_else(invalid)?;
            if layer.space == "source" && !range(layer.offset, layer.length, parent.length)
                || layer.space != "source" && (layer.offset != 0 || layer.length > MAX_OUTPUT)
            {
                return Err(invalid());
            }
            depths.get(&parent.id).copied().ok_or_else(invalid)? + 1
        } else {
            if number != 0
                || layer.space != "source"
                || layer.offset != 0
                || layer.length != size
                || layer.input_sha256 != hash
            {
                return Err(invalid());
            }
            0
        };
        if depth > 7 {
            return Err(invalid());
        }
        index.insert(layer.id.as_str(), layer);
        depths.insert(layer.id.clone(), depth);
    }
    if layers[0].completeness != completeness {
        return Err(invalid());
    }
    for gap in gaps {
        let layer = index.get(gap.layer_id.as_str()).ok_or_else(invalid)?;
        if !range(gap.offset, gap.length, layer.length) || !bounded_text(&gap.reason, 1024) {
            return Err(invalid());
        }
    }
    let expected: Vec<ByteRegion> = gaps
        .iter()
        .filter(|gap| gap.layer_id == layers[0].id)
        .map(|gap| ByteRegion {
            offset: gap.offset,
            length: gap.length,
        })
        .collect();
    if report.unextracted_regions != expected {
        return Err(invalid());
    }
    for layer in layers {
        let has_gap = gaps.iter().any(|gap| gap.layer_id == layer.id);
        let incomplete_child = layers.iter().any(|child| {
            child.parent_id.as_deref() == Some(&layer.id) && child.completeness != "complete"
        });
        if layer.completeness == "complete"
            && (has_gap || incomplete_child || !layer.reasons.is_empty())
        {
            return Err(invalid());
        }
    }
    let status = if completeness == "complete" {
        "extracted_verified"
    } else if report.file_count > 0 {
        "partially_extracted"
    } else if completeness == "partial" {
        "inspection_only"
    } else if completeness == "unsupported" {
        "unsupported"
    } else {
        "encrypted-or-unknown"
    };
    if report.status != status {
        return Err(invalid());
    }
    for window in entropy {
        if !range(window.offset, window.length, size)
            || window.length > 1024 * 1024
            || window.sample_bytes > 65536
            || window.sample_bytes == 0
            || !window.bits_per_byte.is_finite()
            || !(0.0..=8.0).contains(&window.bits_per_byte)
            || window.classification
                != if window.bits_per_byte >= 7.6 {
                    "high_entropy_compressed_or_encrypted"
                } else {
                    "mixed_or_low_entropy"
                }
        {
            return Err(invalid());
        }
    }
    for entry in omitted {
        if !index.contains_key(entry.layer_id.as_str())
            || !safe_name(&entry.path)
            || !bounded_text(&entry.reason, 1024)
        {
            return Err(invalid());
        }
    }
    for summary in swupdate {
        if !index.contains_key(summary.layer_id.as_str())
            || summary.authority != "package_declarations_unverified"
            || summary.signature_verified
            || summary.versions.len() > 32
            || summary.versions.iter().any(|s| !bounded_text(s, 128))
            || summary.declarations.len() > 256
        {
            return Err(invalid());
        }
        for item in &summary.declarations {
            if [
                &item.filename,
                &item.kind,
                &item.sha256,
                &item.size,
                &item.name,
            ]
            .iter()
            .any(|v| v.as_ref().is_some_and(|s| !bounded_text(s, 512)))
                || item.base_requirements.as_ref().is_some_and(|map| {
                    map.len() > 256
                        || map
                            .iter()
                            .any(|(k, v)| !bounded_text(k, 128) || !bounded_text(v, 512))
                })
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
