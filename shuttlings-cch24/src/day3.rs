use axum::{body::Bytes, http::StatusCode};
use axum_extra::TypedHeader;
use cargo_manifest::{Manifest, Package};
use headers::ContentType;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug)]
struct Orders {
    #[serde(default)]
    orders: Option<Vec<Order>>,
}

#[derive(Deserialize, Debug)]
struct Order {
    #[serde(default)]
    item: Option<String>,

    #[serde(default, deserialize_with = "deserialize_quantity")]
    quantity: Option<u32>,
}

#[allow(clippy::unnecessary_wraps)]
fn deserialize_quantity<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let result = u32::deserialize(deserializer);
    match result {
        Ok(v) => Ok(Some(v)),
        Err(_) => Ok(None), // If deserialization fails, return None
    }
}

// Ensure the magic keyword is present in the keywords
fn keyword_present(package: &Package<Orders>) -> bool {
    let magic_word = "Christmas 2024".to_string();

    package
        .keywords
        .as_ref()
        .and_then(|keywords| keywords.clone().as_local())
        .and_then(|keywords| keywords.contains(&magic_word).then_some(true))
        .is_some()
}

/// Parse the given toml bytes as a [`Manifest`]
fn parse_manifest_bytes(toml_bytes: &[u8]) -> Result<String, (StatusCode, String)> {
    let manifest: Manifest<Orders> = Manifest::from_slice_with_metadata(toml_bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid manifest".to_string()))?;

    let Some(package) = &manifest.package else {
        return Err((StatusCode::BAD_REQUEST, "Invalid manifest".to_string()));
    };

    // Ensure the 'Christmas 2024' keyword is added
    if !keyword_present(package) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Magic keyword not provided".to_string(),
        ));
    }

    let Some(orders) = &package.metadata else {
        return Err((StatusCode::NO_CONTENT, String::new()));
    };

    let Some(orders) = &orders.orders else {
        return Err((StatusCode::NO_CONTENT, String::new()));
    };

    // Collect orders together
    let result = orders
        .iter()
        .filter_map(|Order { item, quantity }| match (item, quantity) {
            (Some(item), Some(quantity)) => Some(format!("{item}: {quantity}")),
            _ => None,
        })
        .collect::<Vec<_>>();

    // Ensure we have some orders
    if result.is_empty() {
        return Err((StatusCode::NO_CONTENT, String::new()));
    }

    // Return the list of orders
    Ok(result.join("\n"))
}

pub async fn manifest(
    TypedHeader(content_type): TypedHeader<ContentType>,
    body: Bytes,
) -> Result<String, (StatusCode, String)> {
    match content_type.to_string().as_str() {
        "application/toml" => parse_manifest_bytes(&body),
        "application/yaml" => {
            let yaml: serde_json::Value = serde_yaml::from_slice(&body)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid manifest".to_string()))?;

            let toml = toml::to_string(&yaml)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to create toml".to_string()))?;

            parse_manifest_bytes(toml.as_bytes())
        }
        "application/json" => {
            let json: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid manifest".to_string()))?;

            let toml = toml::to_string(&json)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to create toml".to_string()))?;

            parse_manifest_bytes(toml.as_bytes())
        }
        x => Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            format!("Unknown content type: {x}"),
        )),
    }
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
    use tower::util::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn manifest() {
        let app = app();

        let data = r#"
[package]
name = "not-a-gift-order"
authors = ["Not Santa"]
keywords = ["Christmas 2024"]

[[package.metadata.orders]]
item = "Toy car"
quantity = 2

[[package.metadata.orders]]
item = "Lego brick"
quantity = 230
"#;

        let response = app
            .oneshot(
                Request::post("/5/manifest".to_string())
                    .header(header::CONTENT_TYPE, "application/toml")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "Toy car: 2\nLego brick: 230");
    }

    #[tokio::test]
    async fn manifest_with_float() {
        let app = app();

        let data = r#"
    [package]
    name = "not-a-gift-order"
    authors = ["Not Santa"]
    keywords = ["Christmas 2024"]

    [[package.metadata.orders]]
    item = "Toy car"
    quantity = 2

    [[package.metadata.orders]]
    item = "Lego brick"
    quantity = 230

    [[package.metadata.orders]]
    item = "Invalid"
    quantity = 230.5
    "#;

        let response = app
            .oneshot(
                Request::post("/5/manifest".to_string())
                    .header(header::CONTENT_TYPE, "application/toml")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "Toy car: 2\nLego brick: 230");
    }

    #[tokio::test]
    async fn manifest_bad_data() {
        let app = app();

        let data = r#"
    [package]
    name = "coal-in-a-bowl"
    authors = ["H4CK3R_13E7"]
    keywords = ["Christmas 2024"]

    [[package.metadata.orders]]
    item = "Coal"
    quantity = "Hahaha get rekt"
    "#;

        let response = app
            .oneshot(
                Request::post("/5/manifest".to_string())
                    .header(header::CONTENT_TYPE, "application/toml")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn manifest_bad_manifest() {
        let app = app();

        let data = r#"
    [package]
    name = false
    authors = ["Not Santa"]
    keywords = ["Christmas 2024"]
    "#;

        let response = app
            .oneshot(
                Request::post("/5/manifest".to_string())
                    .header(header::CONTENT_TYPE, "application/toml")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn manifest_bad_manifest2() {
        let app = app();

        let data = r#"
    [package]
    name = "not-a-gift-order"
    authors = ["Not Santa"]
    keywords = ["Christmas 2024"]

    [profile.release]
    incremental = "stonks"
    "#;

        let response = app
            .oneshot(
                Request::post("/5/manifest".to_string())
                    .header(header::CONTENT_TYPE, "application/toml")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn manifest_missing_magic_keyword() {
        let app = app();

        let data = r#"
    [package]
    name = "grass"
    authors = ["A vegan cow"]
    keywords = ["Moooooo"]
    "#;

        let response = app
            .oneshot(
                Request::post("/5/manifest".to_string())
                    .header(header::CONTENT_TYPE, "application/toml")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "Magic keyword not provided");
    }

    #[tokio::test]
    async fn manifest_yaml() {
        let app = app();

        let data = r#"
    package:
      name: big-chungus-sleigh
      version: "2.0.24"
      metadata:
        orders:
          - item: "Toy train"
            quantity: 5
      rust-version: "1.69"
      keywords:
        - "Christmas 2024"
    "#;

        let response = app
            .oneshot(
                Request::post("/5/manifest".to_string())
                    .header(header::CONTENT_TYPE, "application/yaml")
                    .body(Body::from(data))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "Toy train: 5");
    }
}
