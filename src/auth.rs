use crate::errors::ServiceError;
use alcoholic_jwt::{token_kid, validate, Validation, JWKS};
use serde::{Deserialize, Serialize};
use std;

use actix_web::dev::ServiceRequest;

use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    company: String,
    exp: usize,
}

pub async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, actix_web::Error> {
    let config = req
        .app_data::<Config>()
        .map(|data| data.clone())
        .unwrap_or_else(Default::default);
    match validate_token(credentials.token()).await {
        Ok(res) => {
            if res == true {
                Ok(req)
            } else {
                Err(AuthenticationError::from(config).into())
            }
        }
        Err(_) => Err(AuthenticationError::from(config).into()),
    }
}

pub async fn validate_token(token: &str) -> Result<bool, ServiceError> {
    let authority = std::env::var("AUTHORITY").expect("AUTHORITY must be set");
    let audience = std::env::var("AUDIENCE").expect("AUDIENCE must be set");

    let jwks = fetch_jwks(&format!(
        "{}{}",
        authority.as_str(),
        "/.well-known/jwks.json"
    ))
    .await
    .unwrap();

    let validations = vec![
        Validation::Issuer(authority),
        Validation::Audience(audience),
        Validation::NotExpired,
        Validation::SubjectPresent,
    ];
    let kid = match token_kid(&token) {
        Ok(res) => res.expect("failed to decode kid"),
        Err(_) => return Err(ServiceError::JWKSFetchError),
    };
    let jwk = jwks.find(&kid).expect("Specified key not found in set");
    let res = validate(token, jwk, validations);
    Ok(res.is_ok())
}

async fn fetch_jwks(uri: &str) -> Result<JWKS, Box<dyn std::error::Error>> {
    let val = reqwest::get(uri).await?.json::<JWKS>().await;
    // let val = async {res.json::<JWKS>().await};
    return Ok(val.unwrap());
}
