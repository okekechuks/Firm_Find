//! Synthetic challenges; no socket or physical device is used.
use super::*;

fn credentials() -> Credentials {
    Credentials::new("Mufasa".into(), "Circle of Life".into()).unwrap()
}
fn route() -> VerifiedReadRoute {
    VerifiedReadRoute {
        uri: "/dir/index.html",
    }
}

#[test]
fn digest_rfc2617_vector_preserves_qop_opaque_and_nonce_count() {
    let challenge = r#"Digest realm="testrealm@host.com", qop="auth,auth-int", nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093", opaque="5ccc069c403ebaf9f0171e9517f40e41""#;
    // RFC 2617 uses capital O; RFC 7616's later example uses lowercase o.
    let mut auth = ManagementAuthenticator::digest(
        challenge,
        Credentials::new("Mufasa".into(), "Circle Of Life".into()).unwrap(),
    )
    .unwrap();
    let header = auth
        .authorization_inner(&route(), Some("0a4f113b"))
        .unwrap();
    assert!(header.starts_with("Digest "));
    assert!(header.contains("qop=auth"));
    assert!(header.contains("nc=00000001"));
    assert!(header.contains(r#"opaque="5ccc069c403ebaf9f0171e9517f40e41""#));
    assert!(header.contains(r#"response="6629fae49393a05397450978507c4ef1""#));
    assert!(auth
        .authorization(&route())
        .unwrap()
        .contains("nc=00000002"));
}

#[test]
fn digest_rfc7616_sha256_exact_vector() {
    let challenge = r#"Digest realm="http-auth@example.org",qop="auth,auth-int",algorithm=SHA-256,nonce="7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v",opaque="FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS""#;
    let mut auth = ManagementAuthenticator::digest(challenge, credentials()).unwrap();
    let header = auth
        .authorization_inner(
            &route(),
            Some("f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ"),
        )
        .unwrap();
    assert!(header.contains(
        r#"response="753927fa0e85d155564e2e272a28d1802ca10daf4496794697cf8db5856cb6c1""#
    ));
}

#[test]
fn digest_sha512_256_utf8_userhash_matches_independent_fips_vector() {
    let challenge = r#"Digest realm="api@example.org",qop="auth",algorithm=SHA-512-256,nonce="5TsQWLVdgBdmrQ0XsxbDODV+57QdFR34I9HAbC/RVvkK",opaque="HRPCssKJSGjCrkzDg8OhwpzCiGPChXYjwrI2QmXDnsOS",charset=UTF-8,userhash=true"#;
    let credentials =
        Credentials::new("J\u{00e4}s\u{00f8}n Doe".into(), "Secret, or not?".into()).unwrap();
    let mut auth = ManagementAuthenticator::digest(challenge, credentials).unwrap();
    let header = auth
        .authorization_inner(
            &VerifiedReadRoute { uri: "/doe.json" },
            Some("NTg6RKcb9boFIAS3KrFK9BGeh+iDa/sm6jUMp2wds69v"),
        )
        .unwrap();
    // RFC 7616's printed SHA-512-256 example has reported erratum 4897.
    // This FIPS SHA-512/256 result was independently checked with Python hashlib.
    assert!(header.contains(
        r#"response="3798d4131c277846293534c3edc11bd8a5e4cdcbff78b05db9d95eeb1cec68a5""#
    ));
    assert!(!header.contains("Doe"));
}

#[test]
fn digest_legacy_session_algorithms_and_auth_int_are_explicit() {
    for algorithm in [
        "MD5",
        "MD5-sess",
        "SHA-256",
        "SHA-256-sess",
        "SHA-512-256",
        "SHA-512-256-sess",
    ] {
        let mut auth = ManagementAuthenticator::digest(
            &format!(
                r#"Digest realm="synthetic",nonce="first",algorithm={algorithm},qop="auth-int""#
            ),
            credentials(),
        )
        .unwrap();
        let header = auth.authorization(&route()).unwrap();
        assert!(header.contains("qop=auth-int"), "{algorithm}");
        assert!(header.contains("cnonce="));
    }
    let mut legacy = ManagementAuthenticator::digest(
        r#"Digest realm="synthetic",nonce="legacy""#,
        credentials(),
    )
    .unwrap();
    assert!(!legacy.authorization(&route()).unwrap().contains("qop="));
}

#[test]
fn digest_session_algorithm_without_qop_is_rejected_instead_of_omitting_cnonce() {
    for algorithm in ["MD5-sess", "SHA-256-sess", "SHA-512-256-sess"] {
        assert!(matches!(
            ManagementAuthenticator::digest(
                &format!(r#"Digest realm="synthetic",nonce="first",algorithm={algorithm}"#),
                credentials(),
            ),
            Err(AuthError::UnsupportedScheme)
        ));
    }
}

#[test]
fn digest_stale_nonce_reset_cannot_change_realm_algorithm_or_reuse_nonce() {
    let original = r#"Digest realm="synthetic",nonce="first",algorithm=SHA-256,qop="auth""#;
    let mut auth = ManagementAuthenticator::digest(original, credentials()).unwrap();
    for fresh in [
        r#"Digest realm="synthetic",nonce="first",algorithm=SHA-256,qop="auth",stale=true"#,
        r#"Digest realm="other",nonce="second",algorithm=SHA-256,qop="auth",stale=true"#,
        r#"Digest realm="synthetic",nonce="second",algorithm=MD5,qop="auth",stale=true"#,
        r#"Digest realm="synthetic",nonce="second",algorithm=SHA-256,qop="auth""#,
    ] {
        assert_eq!(
            auth.replace_stale_challenge(fresh),
            Err(AuthError::AuthenticationFailed)
        );
    }
    auth.replace_stale_challenge(
        r#"Digest realm="synthetic",nonce="second",algorithm=SHA-256,qop="auth",stale=true"#,
    )
    .unwrap();
    assert!(auth
        .authorization(&route())
        .unwrap()
        .contains("nc=00000001"));
    auth.challenge.0.nc = u32::MAX;
    assert!(matches!(
        auth.authorization(&route()),
        Err(AuthError::NonceExhausted)
    ));
}

#[test]
fn digest_sha256_and_actual_query_are_supported() {
    let mut auth = ManagementAuthenticator::digest(
        r#"Digest realm="synthetic", nonce="server-nonce", algorithm=SHA-256, qop="auth""#,
        credentials(),
    )
    .unwrap();
    let header = auth
        .authorization(&VerifiedReadRoute {
            uri: "/status?view=identity",
        })
        .unwrap();
    assert!(header.contains("algorithm=SHA-256"));
    assert!(header.contains(r#"uri="/status?view=identity""#));
    assert!(!header.contains("Circle of Life"));
}

#[test]
fn malformed_challenges_fail_closed() {
    for value in [
        "",
        "Basic realm=synthetic",
        "Digest realm=\"test\"",
        "Digest realm=\"a\",nonce=\"x\",algorithm=SHA-1",
        "Digest realm=\"a\",nonce=\"x\",qop=\"invalid\"",
    ] {
        assert!(
            ManagementAuthenticator::digest(value, credentials()).is_err(),
            "{value}"
        );
    }
}

#[test]
fn observed_family_cannot_create_verified_read_route() {
    for family in ["asr_duster_xml", "generic_digest", "openwrt", "unknown"] {
        assert!(matches!(
            VerifiedReadRoute::for_family(family),
            Err(AuthError::UnqualifiedRoute)
        ));
    }
}
