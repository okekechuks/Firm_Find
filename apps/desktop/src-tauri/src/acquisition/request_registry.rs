//! Closed metadata-only request table. This module validates descriptions and never dispatches.
use std::{collections::BTreeMap, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProtocolFamilyId {
    AsrDusterXml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequestId {
    Root,
    StaticVersion,
    StaticMenu,
    Login,
    Logout,
    XmlAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    Html,
    PlainText,
    Xml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserId {
    RootHtml,
    StaticVersionXml,
    StaticMenuXml,
}

/// Closed table entries have private fields and no caller-facing constructor.
///
/// ```compile_fail,E0451
/// use obtainer_lib::acquisition::request_registry::{RequestId, RequestSpec};
/// let _ = RequestSpec {
///     id: RequestId::Login,
///     method: "GET",
///     canonical_path: "/login.cgi",
///     response_types: &[],
///     max_bytes: 16,
///     parser_id: obtainer_lib::acquisition::request_registry::ParserId::RootHtml,
///     max_attempts: 1,
///     budget: std::time::Duration::from_secs(1),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSpec {
    id: RequestId,
    method: &'static str,
    canonical_path: &'static str,
    response_types: &'static [ResponseType],
    max_bytes: usize,
    parser_id: ParserId,
    max_attempts: u8,
    budget: Duration,
}

impl RequestSpec {
    pub fn id(&self) -> RequestId {
        self.id
    }
    pub fn method(&self) -> &'static str {
        self.method
    }
    pub fn canonical_path(&self) -> &'static str {
        self.canonical_path
    }
    pub fn response_types(&self) -> &'static [ResponseType] {
        self.response_types
    }
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }
    pub fn parser_id(&self) -> ParserId {
        self.parser_id
    }
    pub fn max_attempts(&self) -> u8 {
        self.max_attempts
    }
    pub fn budget(&self) -> Duration {
        self.budget
    }
}

const ROOT_TYPES: &[ResponseType] = &[ResponseType::Html];
const VERSION_TYPES: &[ResponseType] = &[ResponseType::PlainText, ResponseType::Xml];
const DENIED_WORDS: &[&str] = &[
    "set", "upload", "upgrade", "reboot", "restore", "reset", "factory", "download", "delete",
    "format", "flash",
];

const ASR_DUSTER_XML_SPECS: &[RequestSpec] = &[
    RequestSpec {
        id: RequestId::Root,
        method: "GET",
        canonical_path: "/",
        response_types: ROOT_TYPES,
        max_bytes: 16 * 1024,
        parser_id: ParserId::RootHtml,
        max_attempts: 1,
        budget: Duration::from_secs(5),
    },
    RequestSpec {
        id: RequestId::StaticVersion,
        method: "GET",
        canonical_path: "/version_num.txt",
        response_types: VERSION_TYPES,
        max_bytes: 4 * 1024,
        parser_id: ParserId::StaticVersionXml,
        max_attempts: 1,
        budget: Duration::from_secs(5),
    },
    RequestSpec {
        id: RequestId::StaticMenu,
        method: "GET",
        canonical_path: "/xml/ui_mifi.xml",
        response_types: &[ResponseType::Xml],
        max_bytes: 16384,
        parser_id: ParserId::StaticMenuXml,
        max_attempts: 1,
        budget: Duration::from_secs(5),
    },
];

#[derive(Debug, Clone, Copy)]
pub struct RequestEnvelope<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub body: Option<&'a [u8]>,
    pub redirect: Option<&'a str>,
}

impl<'a> RequestEnvelope<'a> {
    pub fn get(url: &'a str) -> Self {
        Self {
            method: "GET",
            url,
            body: None,
            redirect: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestValidationError {
    UnregisteredRequest,
    InvalidOrigin,
    InvalidUrl,
    MethodDenied,
    BodyDenied,
    EncodedPathDenied,
    TraversalDenied,
    UserInfoDenied,
    FragmentDenied,
    AlternateHostDenied,
    RedirectDenied,
    DeniedOperation,
    CallerPathDenied,
    QueryDenied,
}

#[derive(Debug)]
pub struct RequestRegistry {
    families: BTreeMap<ProtocolFamilyId, &'static [RequestSpec]>,
}

impl Default for RequestRegistry {
    fn default() -> Self {
        let mut families = BTreeMap::new();
        families.insert(ProtocolFamilyId::AsrDusterXml, ASR_DUSTER_XML_SPECS);
        Self { families }
    }
}

impl RequestRegistry {
    pub fn registered(&self, family: ProtocolFamilyId) -> &'static [RequestSpec] {
        self.families.get(&family).copied().unwrap_or(&[])
    }

    pub fn lookup(&self, family: ProtocolFamilyId, id: RequestId) -> Option<&'static RequestSpec> {
        self.registered(family).iter().find(|spec| spec.id == id)
    }

    pub fn validate(
        &self,
        family: ProtocolFamilyId,
        id: RequestId,
        origin: &str,
        request: &RequestEnvelope<'_>,
    ) -> Result<&'static RequestSpec, RequestValidationError> {
        let spec = self
            .lookup(family, id)
            .ok_or(RequestValidationError::UnregisteredRequest)?;
        if request.method != spec.method {
            return Err(RequestValidationError::MethodDenied);
        }
        if request.body.is_some() {
            return Err(RequestValidationError::BodyDenied);
        }

        let base = url::Url::parse(origin).map_err(|_| RequestValidationError::InvalidOrigin)?;
        if !base.path().eq("/")
            || !matches!(base.scheme(), "http" | "https")
            || base.query().is_some()
            || base.fragment().is_some()
            || !base.username().is_empty()
            || base.password().is_some()
        {
            return Err(RequestValidationError::InvalidOrigin);
        }
        let target =
            url::Url::parse(request.url).map_err(|_| RequestValidationError::InvalidUrl)?;
        if !target.username().is_empty() || target.password().is_some() {
            return Err(RequestValidationError::UserInfoDenied);
        }
        if target.fragment().is_some() {
            return Err(RequestValidationError::FragmentDenied);
        }
        if !same_origin(&base, &target) {
            return Err(RequestValidationError::AlternateHostDenied);
        }

        let (raw_path, raw_query) =
            raw_path_and_query(request.url).ok_or(RequestValidationError::InvalidUrl)?;
        if raw_path.contains('%')
            || raw_query.is_some_and(|query| query.contains('%'))
            || raw_path.contains('\\')
        {
            return Err(RequestValidationError::EncodedPathDenied);
        }
        if raw_path.split('/').any(|part| matches!(part, "." | "..")) {
            return Err(RequestValidationError::TraversalDenied);
        }
        let decoded_for_deny = format!("{}{}", target.path(), target.query().unwrap_or(""));
        if DENIED_WORDS
            .iter()
            .any(|word| decoded_for_deny.to_ascii_lowercase().contains(word))
        {
            return Err(RequestValidationError::DeniedOperation);
        }
        if let Some(location) = request.redirect {
            let redirected = target
                .join(location)
                .map_err(|_| RequestValidationError::InvalidUrl)?;
            if !same_origin(&base, &redirected) {
                return Err(RequestValidationError::AlternateHostDenied);
            }
            return Err(RequestValidationError::RedirectDenied);
        }
        if raw_query.is_some() || target.query().is_some() {
            return Err(RequestValidationError::QueryDenied);
        }
        if raw_path != spec.canonical_path || target.path() != spec.canonical_path {
            return Err(RequestValidationError::CallerPathDenied);
        }
        Ok(spec)
    }
}

fn same_origin(left: &url::Url, right: &url::Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn raw_path_and_query(input: &str) -> Option<(&str, Option<&str>)> {
    let (_, after_scheme) = input.split_once("://")?;
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let suffix = &after_scheme[authority_end..];
    let suffix = suffix.split_once('#').map_or(suffix, |(before, _)| before);
    let (path, query) = suffix
        .split_once('?')
        .map_or((suffix, None), |(path, query)| (path, Some(query)));
    let path = if path.is_empty() { "/" } else { path };
    Some((path, query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_specs_are_bounded_and_read_only() {
        let specs = RequestRegistry::default().registered(ProtocolFamilyId::AsrDusterXml);
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].method(), "GET");
        assert_eq!(specs[0].max_bytes(), 16 * 1024);
        assert_eq!(specs[1].method(), "GET");
        assert_eq!(specs[1].max_bytes(), 4 * 1024);
        assert!(specs
            .iter()
            .all(|spec| spec.max_attempts() == 1 && spec.budget() <= Duration::from_secs(5)));
    }
}
