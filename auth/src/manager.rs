use crate::{errors::AuthError, types::TokenClaims};
use async_trait::async_trait;
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, TokenData, Validation,
};
use opentelemetry::Context;
use serde_json::Value;
use std::{collections::HashMap, str::FromStr};
use tracing::error;

#[async_trait]
pub trait JwtManager: Send + Sync {
    async fn verify(&self, ctx: &Context, token: &str) -> Result<TokenClaims, AuthError>;

    fn decode_token(
        &self,
        token: &str,
        jwks: &JwkSet,
        aud: &str,
        iss: &str,
    ) -> Result<TokenData<HashMap<String, Value>>, AuthError> {
        let Ok(header) = decode_header(token) else {
            error!("failed to decoded token header");
            return Err(AuthError::InvalidToken(
                "failed to decoded token header".into(),
            ));
        };

        let Some(kid) = header.kid else {
            error!("token header without kid");
            return Err(AuthError::InvalidToken("token header without kid".into()));
        };

        let Some(jwk) = jwks.find(&kid) else {
            error!("wasn't possible to find the some token kid into jwks");
            return Err(AuthError::InvalidToken(
                "wasn't possible to find the some token kid into jwks".into(),
            ));
        };

        let AlgorithmParameters::RSA(rsa) = &jwk.algorithm else {
            error!("token hashed using other algorithm than RSA");
            return Err(AuthError::InvalidToken(
                "token hashed using other algorithm than RSA".into(),
            ));
        };

        let Ok(decoding_key) = DecodingKey::from_rsa_components(&rsa.n, &rsa.e) else {
            error!("failed to decode rsa components");
            return Err(AuthError::InvalidToken(
                "failed to decode rsa components".into(),
            ));
        };

        let Some(key_alg) = jwk.common.key_algorithm else {
            error!("jwk with no key algorithm");
            return Err(AuthError::InvalidToken("jwk with no key algorithm".into()));
        };

        let Ok(alg) = Algorithm::from_str(key_alg.to_string().as_str()) else {
            error!("algorithm provided by the JWK is not sported!");
            return Err(AuthError::InvalidToken(
                "algorithm provided by the JWK is not sported!".into(),
            ));
        };

        let mut validation = Validation::new(alg);

        validation.set_audience(&[aud]);
        validation.set_issuer(&[iss]);
        validation.validate_exp = true;

        match decode::<HashMap<String, Value>>(token, &decoding_key, &validation) {
            Ok(d) => Ok(d),
            Err(err) => {
                error!(error = err.to_string(), "token validation error");
                Err(AuthError::InvalidToken("token validation error".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use openssl::rsa::Rsa;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize)]
    struct MockedClaims {
        sub: String,
        company: String,
        exp: usize,
    }

    impl MockedClaims {
        pub fn new() -> Self {
            Self {
                sub: "1234567890".to_owned(),
                company: "ACME".to_owned(),
                exp: 10000000000,
            }
        }
    }

    struct MyJwtManager;
    #[async_trait]
    impl JwtManager for MyJwtManager {
        async fn verify(&self, _ctx: &Context, _token: &str) -> Result<TokenClaims, AuthError> {
            todo!()
        }
    }

    struct SetupTests {
        pub manager: Box<dyn JwtManager>,
        pub jwk_set: JwkSet,
        pub claims: MockedClaims,
        pub token: String,
    }

    impl SetupTests {
        pub fn new(without_kid: bool, wrong_algorithm: bool) -> Self {
            let claims = MockedClaims::new();

            let rsa = Rsa::generate(2048).unwrap();
            let private_key_pem = rsa.private_key_to_pem().unwrap();
            let _public_key_pem = rsa.public_key_to_pem().unwrap();

            let n = base64::encode_config(rsa.n().to_vec(), base64::URL_SAFE_NO_PAD);
            let e = base64::encode_config(rsa.e().to_vec(), base64::URL_SAFE_NO_PAD);

            // Construct the JWK
            let serialized = json!({
                "keys": [{
                    "kty": "RSA",
                    "n": n,
                    "e": e,
                    "use": "sig",
                    "alg": "RS256",
                    "kid": "key1"
                }]
            });

            let jwk_set: JwkSet = serde_json::from_value(serialized).unwrap();

            let encoding_key = EncodingKey::from_rsa_pem(&private_key_pem).unwrap();
            let token = encode(
                &Header {
                    typ: Some("JWT".into()),
                    alg: Self::alg(wrong_algorithm),
                    cty: None,
                    jku: None,
                    jwk: Some(jwk_set.keys[0].clone()),
                    kid: Self::kid(without_kid),
                    x5u: None,
                    x5c: None,
                    x5t: None,
                    x5t_s256: None,
                },
                &claims,
                &encoding_key,
            )
            .unwrap();

            let manager = Box::new(MyJwtManager);

            Self {
                manager,
                jwk_set,
                claims,
                token,
            }
        }

        fn kid(without: bool) -> Option<String> {
            if without {
                return None;
            }

            return Some("key1".into());
        }

        fn alg(wrong: bool) -> jsonwebtoken::Algorithm {
            if wrong {
                return jsonwebtoken::Algorithm::RS512;
            }

            return jsonwebtoken::Algorithm::RS256;
        }
    }

    #[test]
    fn test_decode_token() {
        let sut = SetupTests::new(false, false);

        let aud = "aud";
        let iss = "iss";

        let token_data = sut.manager.decode_token(&sut.token, &sut.jwk_set, aud, iss);
        assert!(token_data.is_ok());
        let token_data = token_data.unwrap();

        let sub = token_data.claims.get("sub");
        assert!(sub.is_some());
        let sub = sub.unwrap().as_str().unwrap().to_string();

        let company = token_data.claims.get("company");
        assert!(company.is_some());
        let company = company.unwrap().as_str().unwrap().to_string();

        let exp = token_data.claims.get("exp");
        assert!(exp.is_some());
        let exp = exp.unwrap().as_i64().unwrap() as usize;

        assert_eq!(sub, sut.claims.sub);
        assert_eq!(company, sut.claims.company);
        assert_eq!(exp, sut.claims.exp);
    }

    #[test]
    fn test_decode_token_with_invalid_token() {
        let sut = SetupTests::new(false, false);

        let token_data =
            sut.manager
                .decode_token("invalid token", &sut.jwk_set, "aud".into(), "iss".into());

        assert!(token_data.is_err());
        assert_eq!(
            token_data.unwrap_err(),
            AuthError::InvalidToken("failed to decoded token header".into())
        );
    }

    #[test]
    fn test_decode_token_with_invalid_header() {
        let token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.";
        let sut = SetupTests::new(false, false);

        let token_data = sut
            .manager
            .decode_token(token, &sut.jwk_set, "aud".into(), "iss".into());

        assert!(token_data.is_err());
        assert_eq!(
            token_data.unwrap_err(),
            AuthError::InvalidToken("failed to decoded token header".into())
        );
    }

    #[test]
    fn test_decode_token_with_invalid_kid() {
        let sut = SetupTests::new(true, false);

        let token_data =
            sut.manager
                .decode_token(&sut.token, &sut.jwk_set, "aud".into(), "iss".into());

        assert!(token_data.is_err());
        assert_eq!(
            token_data.unwrap_err(),
            AuthError::InvalidToken("token header without kid".into())
        );
    }

    #[test]
    fn test_decode_token_with_wrong_algorithm() {
        let sut = SetupTests::new(false, true);

        let token_data =
            sut.manager
                .decode_token(&sut.token, &sut.jwk_set, "aud".into(), "iss".into());

        assert!(token_data.is_err());
        assert_eq!(
            token_data.unwrap_err(),
            AuthError::InvalidToken("token validation error".into())
        );
    }
}
