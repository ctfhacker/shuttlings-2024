use axum::{
    body::Bytes,
    extract::State,
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
};
use axum_extra::TypedHeader;
use headers::ContentType;
use jsonwebtoken::{decode_header, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const COOKIE_NAME: &str = "gift";

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Claims {
    exp: i64,
    data: serde_json::Value,
}

impl From<serde_json::Value> for Claims {
    fn from(val: serde_json::Value) -> Self {
        Self {
            exp: 0xdead_beef,
            data: val,
        }
    }
}

pub async fn wrap(
    TypedHeader(content_type): TypedHeader<ContentType>,
    body: Bytes,
) -> Result<HeaderMap, (StatusCode, &'static str)> {
    if !matches!(content_type.to_string().as_str(), "application/json") {
        return Err((StatusCode::BAD_REQUEST, ""));
    }

    let data: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to encode json"))?;

    let token = jsonwebtoken::encode::<Claims>(
        &Header::default(),
        &data.into(),
        &EncodingKey::from_secret("sharedsecret".as_ref()),
    )
    .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to encode JWT"))?;

    let cookie = format!("{COOKIE_NAME}={token}");
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    Ok(headers)
}

pub async fn unwrap(
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> Result<String, (StatusCode, String)> {
    let Some(cookie) = cookies.get(COOKIE_NAME) else {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("No {COOKIE_NAME} cookie found"),
        ));
    };

    let mut validation = Validation::default();
    validation.validate_exp = false;

    let token = jsonwebtoken::decode::<Claims>(
        cookie,
        &DecodingKey::from_secret("sharedsecret".as_ref()),
        &validation,
    )
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to decode JWT: {e}"),
        )
    })?;

    serde_json::to_string(&token.claims.data).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to decode to json: {e}"),
        )
    })
}

pub async fn decode(
    State(key): State<Arc<DecodingKey>>,
    body: Bytes,
) -> Result<String, (StatusCode, String)> {
    let token = String::from_utf8(body.to_vec())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid body: {e:?}")))?;

    let Ok(header) = decode_header(&token) else {
        return Err((StatusCode::BAD_REQUEST, "Invalid header".to_string()));
    };

    let mut validation = Validation::new(header.alg);
    validation.validate_exp = false;
    validation.required_spec_claims.remove("exp");

    let token =
        jsonwebtoken::decode::<serde_json::Value>(&token, &key, &validation).map_err(|e| {
            let code = if matches!(e.kind(), jsonwebtoken::errors::ErrorKind::InvalidSignature) {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_REQUEST
            };

            (code, format!("Failed to decode JWT: {e}"))
        })?;

    serde_json::to_string(&token.claims).map_err(|e| {
        (
            StatusCode::IM_A_TEAPOT,
            format!("Failed to decode to json: {e}"),
        )
    })
}

#[cfg(test)]
mod day6_tests {
    use super::{Claims, COOKIE_NAME};
    use crate::app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http::header;
    use http_body_util::BodyExt;
    use jsonwebtoken::{EncodingKey, Header};
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn wrap() {
        let app = app();

        let data = r#"{"cookie is delicious?":true}"#;

        let response = app
            .clone()
            .oneshot(
                Request::post("/16/wrap")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        let data_json: serde_json::Value = serde_json::from_str(data).unwrap();

        let token = jsonwebtoken::encode::<Claims>(
            &Header::default(),
            &data_json.clone().into(),
            &EncodingKey::from_secret("sharedsecret".as_ref()),
        )
        .unwrap();

        let cookie = format!("{COOKIE_NAME}={token}");

        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        println!("{headers:?}");
        assert_eq!(
            headers
                .get("set-cookie")
                .expect("No gift")
                .to_str()
                .expect("No str"),
            cookie
        );

        let response = app
            .clone()
            .oneshot(
                Request::get("/16/unwrap")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert_eq!(body, data);
    }
}
