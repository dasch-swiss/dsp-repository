use std::fmt;

use base64::Engine as _;
use jsonwebtoken::{Algorithm, EncodingKey, Header};

use crate::ClientError;

/// `exp - iat` of a service token. dsp-api refuses a lifetime above 15 minutes and does not check
/// `jti` for replay, so tokens are short and minted per request.
const TOKEN_LIFETIME_SECS: i64 = 300;

/// The bridge's service identity on one DSP-API server: the name dsp-api trusts the key under, the
/// key's id, and the Ed25519 signing key. `Debug` shows the name and the key id only.
pub struct ServiceIdentity {
    service_name: String,
    kid: String,
    key: EncodingKey,
}

#[derive(serde::Serialize)]
struct Claims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
    jti: String,
}

impl ServiceIdentity {
    /// Takes the signing key as base64 (standard alphabet) of its PKCS#8 DER encoding and signs a
    /// probe token, because `EncodingKey::from_ed_der` does not parse the key: a key that cannot
    /// sign fails here, not at the first refresh. No error repeats the key.
    pub fn new(service_name: &str, kid: &str, base64_pkcs8_der: &str) -> Result<Self, ClientError> {
        if service_name.trim().is_empty() {
            return Err(ClientError::Identity("the service name is blank"));
        }
        if kid.trim().is_empty() {
            return Err(ClientError::Identity("the key id is blank"));
        }
        let der = base64::engine::general_purpose::STANDARD
            .decode(base64_pkcs8_der.trim())
            .map_err(|_| ClientError::Identity("the signing key is not base64"))?;
        let identity = Self {
            service_name: service_name.to_owned(),
            kid: kid.to_owned(),
            key: EncodingKey::from_ed_der(&der),
        };
        identity.token("probe")?;
        Ok(identity)
    }

    /// A fresh `EdDSA` token for `audience`, the server's base URL as configured.
    pub fn token(&self, audience: &str) -> Result<String, ClientError> {
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(self.kid.clone());
        let iat = chrono::Utc::now().timestamp();
        let claims = Claims {
            iss: &self.service_name,
            sub: &self.service_name,
            aud: audience,
            iat,
            exp: iat + TOKEN_LIFETIME_SECS,
            jti: uuid::Uuid::new_v4().to_string(),
        };
        jsonwebtoken::encode(&header, &claims, &self.key)
            .map_err(|_| ClientError::Identity("the signing key is not an Ed25519 PKCS#8 key"))
    }
}

impl fmt::Debug for ServiceIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceIdentity")
            .field("service_name", &self.service_name)
            .field("kid", &self.kid)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use jsonwebtoken::{DecodingKey, Validation};

    use super::*;

    const PRIVATE_KEY: &str = include_str!("../testdata/ed25519-private.b64");
    const PUBLIC_KEY: &str = include_str!("../testdata/ed25519-public.b64");
    const AUDIENCE: &str = "https://api.example.test";

    #[derive(serde::Deserialize)]
    struct Decoded {
        iss: String,
        sub: String,
        aud: String,
        iat: i64,
        exp: i64,
        jti: String,
    }

    fn identity() -> ServiceIdentity {
        ServiceIdentity::new("dsp-repository-test", "test-kid-1", PRIVATE_KEY).unwrap()
    }

    fn verify(token: &str) -> (Header, Decoded) {
        // The public key is SubjectPublicKeyInfo DER; its last 32 bytes are the raw Ed25519 key.
        let spki = base64::engine::general_purpose::STANDARD.decode(PUBLIC_KEY.trim()).unwrap();
        let key = DecodingKey::from_ed_der(&spki[spki.len() - 32..]);
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_audience(&[AUDIENCE]);
        validation.set_required_spec_claims(&["exp", "iat", "aud", "iss", "sub"]);
        let data = jsonwebtoken::decode::<Decoded>(token, &key, &validation).unwrap();
        (data.header, data.claims)
    }

    #[test]
    fn test_token_verifies_with_the_public_key_and_carries_the_contract_claims() {
        let identity = identity();
        let (header, claims) = verify(&identity.token(AUDIENCE).unwrap());

        assert_eq!(header.alg, Algorithm::EdDSA);
        assert_eq!(header.typ.as_deref(), Some("JWT"));
        assert_eq!(header.kid.as_deref(), Some("test-kid-1"));
        assert_eq!(claims.iss, "dsp-repository-test");
        assert_eq!(claims.sub, "dsp-repository-test");
        assert_eq!(claims.aud, AUDIENCE);
        assert_eq!(claims.exp - claims.iat, 300);
        assert!(!claims.jti.trim().is_empty());
    }

    #[test]
    fn test_token_each_call_has_a_fresh_jti() {
        let identity = identity();
        let jtis: HashSet<String> = (0..5).map(|_| verify(&identity.token(AUDIENCE).unwrap()).1.jti).collect();
        assert_eq!(jtis.len(), 5);
    }

    #[test]
    fn test_new_malformed_key_is_an_error_without_the_input() {
        for input in ["not base64 at all!", "c2VjcmV0LWJ1dC1ub3QtYS1rZXk="] {
            let error = ServiceIdentity::new("svc", "kid", input).unwrap_err();
            let shown = format!("{error} {error:?}");
            assert!(matches!(error, ClientError::Identity(_)), "{shown}");
            assert!(!shown.contains(input), "{shown}");
            assert!(!shown.contains("c2VjcmV0"), "{shown}");
        }
    }

    #[test]
    fn test_new_blank_name_or_kid_is_an_error() {
        assert!(ServiceIdentity::new(" ", "kid", PRIVATE_KEY).is_err());
        assert!(ServiceIdentity::new("svc", "", PRIVATE_KEY).is_err());
    }

    #[test]
    fn test_debug_shows_name_and_kid_but_no_key_material() {
        let shown = format!("{:?}", identity());
        assert!(shown.contains("dsp-repository-test"), "{shown}");
        assert!(shown.contains("test-kid-1"), "{shown}");
        let key = PRIVATE_KEY.trim();
        assert!(!shown.contains(&key[key.len() - 16..]), "{shown}");
        assert!(!shown.contains("key"), "{shown}");
    }
}
