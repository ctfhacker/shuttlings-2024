use axum::{body::Bytes, http::StatusCode, Extension};
use axum_extra::TypedHeader;
use headers::ContentType;
use leaky_bucket::RateLimiter;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const LITERS_IN_GALLON: f32 = 3.785_411_8;
pub const PINTS_IN_LITRES: f32 = 1.759_754;

#[cfg(test)]
pub const RATE_LIMIT_INTERVAL: Duration = Duration::from_millis(10);
#[cfg(not(test))]
pub const RATE_LIMIT_INTERVAL: Duration = Duration::from_millis(1000);

#[derive(Serialize, Deserialize, Debug, Default)]
struct Conversion {
    #[serde(skip_serializing_if = "Option::is_none")]
    gallons: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    liters: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    litres: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pints: Option<f32>,
}

impl Conversion {
    pub fn convert(&self) -> Result<Conversion, (StatusCode, &'static str)> {
        match (self.gallons, self.liters, self.litres, self.pints) {
            (Some(gallons), None, None, None) => Ok(Conversion {
                liters: Some(gallons * LITERS_IN_GALLON),
                ..Default::default()
            }),
            (None, Some(liters), None, None) => Ok(Conversion {
                gallons: Some(liters / LITERS_IN_GALLON),
                ..Default::default()
            }),
            (None, None, Some(litres), None) => Ok(Conversion {
                pints: Some(litres * PINTS_IN_LITRES),
                ..Default::default()
            }),
            (None, None, None, Some(pints)) => Ok(Conversion {
                litres: Some(pints / PINTS_IN_LITRES),
                ..Default::default()
            }),
            _ => Err((StatusCode::BAD_REQUEST, "Invalid units")),
        }
    }
}

pub fn create_milk_limiter() -> RateLimiter {
    RateLimiter::builder()
        .initial(5)
        .max(5)
        .interval(RATE_LIMIT_INTERVAL)
        .build()
}

pub async fn milk(
    limiter: Extension<Arc<Mutex<RateLimiter>>>,
    content_type: Option<TypedHeader<ContentType>>,
    body: Bytes,
) -> Result<String, (StatusCode, &'static str)> {
    if !limiter.lock().unwrap().try_acquire(1) {
        return Err((StatusCode::TOO_MANY_REQUESTS, "No milk available\n"));
    }

    if content_type.is_none()
        || !matches!(
            content_type.unwrap().to_string().as_str(),
            "application/json"
        )
    {
        return Ok("Milk withdrawn\n".to_string());
    }

    let items: Conversion =
        serde_json::from_slice(&body).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid"))?;

    let converted = items.convert()?;
    serde_json::to_string(&converted).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid conversion"))
}

pub async fn refill(
    limiter: Extension<Arc<Mutex<RateLimiter>>>,
) -> Result<(), (StatusCode, &'static str)> {
    *limiter.lock().unwrap() = create_milk_limiter();
    Ok(())
}

#[cfg(test)]
mod day3_tests {
    use crate::app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http::header;
    use http_body_util::BodyExt;
    use tower::util::ServiceExt;

    use super::RATE_LIMIT_INTERVAL;

    #[tokio::test]
    async fn milk_test_limit() {
        let app = app();

        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/9/milk".to_string())
                        .body(Body::default())
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(body, "Milk withdrawn\n");
        }

        let response = app
            .clone()
            .oneshot(
                Request::post("/9/milk".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "No milk available\n");
    }

    #[tokio::test]
    async fn milk_liters_to_gallons() {
        let app = app();

        let data = r#"{"liters":5}"#;

        let response = app
            .oneshot(
                Request::post("/9/milk".to_string())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"gallons":1.3208603}"#);
    }

    #[tokio::test]
    async fn milk_gallons_to_liters() {
        let app = app();

        let data = r#"{"gallons":5}"#;

        let response = app
            .oneshot(
                Request::post("/9/milk".to_string())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"liters":18.92706}"#);
    }

    #[tokio::test]
    async fn milk_invalid_data() {
        let app = app();

        let data = r#"{"gallons":5, "liters":1}"#;

        let response = app
            .oneshot(
                Request::post("/9/milk".to_string())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn milk_litres_to_pints() {
        let app = app();

        let data = r#"{"litres":2}"#;

        let response = app
            .oneshot(
                Request::post("/9/milk".to_string())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, r#"{"pints":3.519508}"#);
    }

    #[tokio::test]
    async fn milk_test_refill_with_sleep() {
        let app = app();

        macro_rules! drink_all {
            () => {
                for _ in 0..5 {
                    let response = app
                        .clone()
                        .oneshot(
                            Request::post("/9/milk".to_string())
                                .body(Body::default())
                                .unwrap(),
                        )
                        .await
                        .unwrap();

                    let body = response.into_body().collect().await.unwrap().to_bytes();
                    assert_eq!(body, "Milk withdrawn\n");
                }
            };
        }

        drink_all!();
        std::thread::sleep(RATE_LIMIT_INTERVAL * 5);
        drink_all!();

        let response = app
            .clone()
            .oneshot(
                Request::post("/9/milk".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "No milk available\n");
    }

    #[tokio::test]
    async fn milk_test_refill_with_refill() {
        let app = app();

        macro_rules! drink_all {
            () => {
                for _ in 0..5 {
                    let response = app
                        .clone()
                        .oneshot(
                            Request::post("/9/milk".to_string())
                                .body(Body::default())
                                .unwrap(),
                        )
                        .await
                        .unwrap();

                    let body = response.into_body().collect().await.unwrap().to_bytes();
                    assert_eq!(body, "Milk withdrawn\n");
                }
            };
        }

        drink_all!();

        let response = app
            .clone()
            .oneshot(
                Request::post("/9/refill".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        drink_all!();

        let response = app
            .clone()
            .oneshot(
                Request::post("/9/milk".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "No milk available\n");
    }
}
