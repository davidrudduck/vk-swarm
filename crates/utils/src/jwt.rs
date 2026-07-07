use chrono::{DateTime, Utc};
use jsonwebtoken::dangerous::insecure_decode;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TokenClaimsError {
    #[error("failed to decode JWT: {0}")]
    Decode(#[from] jsonwebtoken::errors::Error),
    #[error("missing `exp` claim in token")]
    MissingExpiration,
    #[error("invalid `exp` value `{0}`")]
    InvalidExpiration(i64),
}

#[derive(Debug, Deserialize)]
struct ExpClaim {
    exp: Option<i64>,
}

/// Extract the expiration timestamp from a JWT without verifying its signature.
pub fn extract_expiration(token: &str) -> Result<DateTime<Utc>, TokenClaimsError> {
    let data = insecure_decode::<ExpClaim>(token)?;
    let exp = data.claims.exp.ok_or(TokenClaimsError::MissingExpiration)?;
    DateTime::from_timestamp(exp, 0).ok_or(TokenClaimsError::InvalidExpiration(exp))
}

#[cfg(test)]
mod tests {
    use super::*;

    use jsonwebtoken::{EncodingKey, Header};

    fn make_jwt_with_exp(exp_delta_secs: i64) -> String {
        #[derive(serde::Serialize)]
        struct Claims {
            exp: usize,
        }

        let exp = (Utc::now() + chrono::Duration::seconds(exp_delta_secs))
            .timestamp() as usize;
        let claims = Claims { exp };
        jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"test-secret"),
        )
        .expect("failed to encode test JWT")
    }

    #[test]
    fn extract_expiration_valid_token() {
        let token = make_jwt_with_exp(3600);
        let result = extract_expiration(&token);
        assert!(result.is_ok());
    }

    #[test]
    fn extract_expiration_future_timestamp() {
        let token = make_jwt_with_exp(3600);
        let exp = extract_expiration(&token).unwrap();
        let now = Utc::now();
        assert!(exp > now);
    }

    #[test]
    fn extract_expiration_past_timestamp() {
        let token = make_jwt_with_exp(-60);
        let exp = extract_expiration(&token).unwrap();
        let now = Utc::now();
        assert!(exp < now);
    }

    #[test]
    fn extract_expiration_invalid_token() {
        let result = extract_expiration("not.a.valid.jwt");
        assert!(result.is_err());
    }

    #[test]
    fn extract_expiration_empty_token() {
        let result = extract_expiration("");
        assert!(result.is_err());
    }

    #[test]
    fn extract_expiration_missing_exp_claim() {
        #[derive(serde::Serialize)]
        struct NoExpClaims {
            sub: String,
        }

        let claims = NoExpClaims {
            sub: "user".into(),
        };
        let token = jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"test-secret"),
        )
        .expect("failed to encode test JWT");

        let result = extract_expiration(&token);
        assert!(matches!(result, Err(TokenClaimsError::MissingExpiration)));
    }
}
