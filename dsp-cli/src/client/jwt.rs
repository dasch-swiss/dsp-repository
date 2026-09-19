//! JWT inspection helpers — extract claims without verifying the signature.
//!
//! Tokens are decoded for metadata (expiry, subject) only. Signature validation
//! is intentionally disabled: we received the token directly from the DSP-API
//! server in the same HTTPS response that authenticated the user. The HTTPS
//! channel is the trust boundary; re-verifying the HMAC/RSA signature here
//! would require knowing the server's key, which dsp-cli does not have and
//! should not need.
//!
//! **Security note on `sub`:** the `sub` claim returned by [`extract_meta`] is
//! **unauthenticated** — the signature is not verified, so this value must be
//! treated as display/metadata only. It must **never** be used as an
//! access-control input. The same caveat applies to the `exp` claim: it is
//! read for display purposes (`dsp auth status` expiry) and is not used to
//! gate access.
//!
//! `dsp auth token` (plan 026) is a **third** consumer of `exp`: it uses the
//! claim as a best-effort local staleness guard — a stale-looking token makes
//! the command exit `3` (`AuthRequired`) instead of printing it. This is
//! explicitly **not** an access-control decision, for the same reason as
//! above: the signature is not verified, so a forged `exp` could lie. It is
//! purely advisory/ergonomic — it saves the caller a failed downstream
//! request by catching an obviously-expired token before it is ever piped
//! into a request. The server remains the sole real gate on access.

#[derive(serde::Deserialize)]
struct MetaClaims {
    exp: Option<i64>,
    sub: Option<String>,
}

/// Metadata extracted from a JWT without verifying the signature.
///
/// Both fields are `Option` because a structurally valid JWT may omit either
/// claim. Absence of a claim is not an error; `extract_meta` returns `None`
/// only when the input is not a parseable JWT at all.
pub(crate) struct JwtMeta {
    pub exp: Option<chrono::DateTime<chrono::Utc>>,
    pub sub: Option<String>,
}

/// Extract metadata from a JWT without verifying the signature.
///
/// Returns `None` if the input is not a structurally valid JWT (i.e.
/// `insecure_decode` fails). Returns `Some(JwtMeta { … })` for any parseable
/// JWT — fields inside `JwtMeta` may individually be `None` if the
/// corresponding claim is absent.
///
/// Signature validation is disabled — see module-level doc. The returned
/// values are for display/metadata only (`dsp auth status`, `set-token`
/// caching). **Do not** use them for any access-control decision.
pub(crate) fn extract_meta(token: &str) -> Option<JwtMeta> {
    // `insecure_decode` performs *zero* validation: no signature check and no
    // claim checks (exp / nbf / aud / algorithm). Because both fields in
    // `MetaClaims` are `Option`, decoding succeeds for any structurally valid
    // JWT and returns `Err` only for non-JWT input (missing/malformed header,
    // base64 errors, etc.).
    jsonwebtoken::dangerous::insecure_decode::<MetaClaims>(token)
        .ok()
        .map(|td| JwtMeta {
            exp: td.claims.exp.and_then(|e| chrono::DateTime::from_timestamp(e, 0)),
            sub: td.claims.sub,
        })
}

/// Extract the `exp` claim from a JWT and return it as a UTC timestamp.
///
/// Returns `None` if the token is malformed, lacks an `exp` claim, or
/// contains an `exp` value that cannot be represented as a `DateTime`.
///
/// Signature validation is disabled — see module-level doc. The return value
/// is for display only (`dsp auth status` expiry). **Do not** use it for any
/// access-control decision: the token is trusted because it arrived over the
/// authenticated HTTPS channel, not because its claims were verified here.
pub(crate) fn extract_exp(token: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    extract_meta(token).and_then(|m| m.exp)
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    use super::*;

    /// Produce a minimal JWT with the given JSON payload using `jsonwebtoken::encode`.
    /// The secret is arbitrary — `extract_exp` disables signature validation.
    fn make_jwt(payload: &serde_json::Value) -> String {
        encode(&Header::new(Algorithm::HS256), payload, &EncodingKey::from_secret(b"unused"))
            .expect("test JWT encoding should not fail")
    }

    // ── extract_exp tests (preserved) ─────────────────────────────────────────

    #[test]
    fn extract_exp_returns_some_for_token_with_exp_claim() {
        // Unix timestamp 1700000000 = 2023-11-14T22:13:20Z
        let token = make_jwt(&serde_json::json!({ "exp": 1700000000_i64 }));
        let result = extract_exp(&token);
        assert!(result.is_some(), "expected Some(DateTime) for token with exp claim");
        let dt = result.unwrap();
        assert_eq!(dt.timestamp(), 1700000000);
    }

    #[test]
    fn extract_exp_returns_none_for_token_without_exp_claim() {
        let token = make_jwt(&serde_json::json!({ "sub": "user@example.com" }));
        let result = extract_exp(&token);
        assert!(result.is_none(), "expected None for token without exp claim");
    }

    #[test]
    fn extract_exp_returns_none_for_garbage_input() {
        assert!(extract_exp("not.a.jwt").is_none());
        assert!(extract_exp("").is_none());
    }

    #[test]
    fn extract_exp_handles_real_world_claim_set_with_aud() {
        // Regression: real DSP-API tokens carry `aud` (a list), plus `iss`,
        // `sub`, `iat`, `nbf`, `jti`, `scope`. jsonwebtoken defaults to
        // validating `aud`/`nbf`, which previously rejected the token and
        // nulled out the expiry. `extract_exp` must read `exp` regardless.
        let token = make_jwt(&serde_json::json!({
            "iss": "https://api.dev.dasch.swiss",
            "sub": "http://rdfh.ch/users/root",
            "aud": ["Knora", "Sipi"],
            "exp": 1782457152_i64,
            "iat": 1779865152_i64,
            "nbf": 1779865152_i64,
            "jti": "abc-123",
            "scope": "admin",
        }));
        let result = extract_exp(&token);
        assert!(
            result.is_some(),
            "expected Some(DateTime) for a real-world token bearing an aud claim"
        );
        assert_eq!(result.unwrap().timestamp(), 1782457152);
    }

    #[test]
    fn extract_exp_accepts_rs256_algorithm_header() {
        // Even though we encode with HS256 internally, the validation's algorithm
        // allow-list is what matters. This test verifies the allow-list is broad
        // enough by checking a token encoded with a different algorithm still
        // parses correctly.
        let token = encode(
            &Header::new(Algorithm::HS384),
            &serde_json::json!({ "exp": 1800000000_i64 }),
            &EncodingKey::from_secret(b"secret"),
        )
        .expect("test JWT encoding should not fail");
        let result = extract_exp(&token);
        assert!(result.is_some());
        assert_eq!(result.unwrap().timestamp(), 1800000000);
    }

    // ── extract_meta tests (new) ───────────────────────────────────────────────

    #[test]
    fn extract_meta_returns_sub_when_present() {
        let token = make_jwt(&serde_json::json!({ "sub": "http://rdfh.ch/users/root", "exp": 1700000000_i64 }));
        let meta = extract_meta(&token).expect("expected Some for valid JWT");
        assert_eq!(
            meta.sub.as_deref(),
            Some("http://rdfh.ch/users/root"),
            "sub should be returned when present"
        );
    }

    #[test]
    fn extract_meta_returns_none_sub_when_absent() {
        let token = make_jwt(&serde_json::json!({ "exp": 1700000000_i64 }));
        let meta = extract_meta(&token).expect("expected Some for valid JWT");
        assert!(meta.sub.is_none(), "sub should be None when not in payload");
    }

    #[test]
    fn extract_meta_returns_both_exp_and_sub_when_both_present() {
        let token = make_jwt(&serde_json::json!({
            "sub": "http://rdfh.ch/users/root",
            "exp": 1782457152_i64,
        }));
        let meta = extract_meta(&token).expect("expected Some for valid JWT");
        assert_eq!(meta.sub.as_deref(), Some("http://rdfh.ch/users/root"));
        assert!(meta.exp.is_some());
        assert_eq!(meta.exp.unwrap().timestamp(), 1782457152);
    }

    #[test]
    fn extract_meta_returns_some_with_all_none_for_empty_payload() {
        // A structurally valid JWT whose payload is `{}` — no recognized claims.
        // extract_meta must return Some (it IS a JWT), with both fields None.
        let token = make_jwt(&serde_json::json!({}));
        let meta = extract_meta(&token).expect("expected Some for valid JWT with empty payload");
        assert!(meta.exp.is_none(), "exp should be None when payload is empty");
        assert!(meta.sub.is_none(), "sub should be None when payload is empty");
    }

    #[test]
    fn extract_meta_returns_none_for_non_jwt_input() {
        assert!(extract_meta("not.a.jwt").is_none(), "non-JWT string should return None");
        assert!(extract_meta("").is_none(), "empty string should return None");
        assert!(extract_meta("garbage").is_none(), "garbage string should return None");
    }

    #[test]
    fn extract_exp_still_returns_none_for_valid_jwt_without_exp() {
        // Regression guard: even after delegating to extract_meta,
        // extract_exp must return None when exp is absent.
        let token = make_jwt(&serde_json::json!({ "sub": "user@example.com" }));
        assert!(
            extract_exp(&token).is_none(),
            "extract_exp must return None for a valid JWT without exp"
        );
    }
}
