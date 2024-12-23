use axum::extract::Query;
use serde::Deserialize;
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Deserialize)]
pub struct DestParams {
    from: Ipv4Addr,
    key: Ipv4Addr,
}

pub async fn ipv4_dest(params: Query<DestParams>) -> String {
    let from = params.from;
    let key = params.key;

    let mut octets = [0u8; 4];
    for (i, (x, y)) in from.octets().iter().zip(key.octets().iter()).enumerate() {
        octets[i] = (*x).wrapping_add(*y);
    }

    Ipv4Addr::from(octets).to_string()
}

#[derive(Deserialize)]
pub struct KeyParams {
    from: Ipv4Addr,
    to: Ipv4Addr,
}

pub async fn ipv4_key(params: Query<KeyParams>) -> String {
    let from = params.from;
    let to = params.to;

    let mut octets = [0u8; 4];
    for (i, (x, y)) in to.octets().iter().zip(from.octets().iter()).enumerate() {
        octets[i] = (*x).wrapping_sub(*y);
    }

    Ipv4Addr::from(octets).to_string()
}

#[derive(Deserialize)]
pub struct Ipv6DestParams {
    from: Ipv6Addr,
    key: Ipv6Addr,
}

#[derive(Deserialize)]
pub struct Ipv6KeyParams {
    from: Ipv6Addr,
    to: Ipv6Addr,
}

fn ipv6_xor(x: Ipv6Addr, y: Ipv6Addr) -> Ipv6Addr {
    let mut octets = [0u8; 16];

    for (i, (x, y)) in x.octets().iter().zip(y.octets().iter()).enumerate() {
        octets[i] = x ^ y;
    }

    Ipv6Addr::from(octets)
}

pub async fn ipv6_dest(params: Query<Ipv6DestParams>) -> String {
    ipv6_xor(params.from, params.key).to_string()
}

pub async fn ipv6_key(params: Query<Ipv6KeyParams>) -> String {
    ipv6_xor(params.from, params.to).to_string()
}

#[cfg(test)]
mod day2_tests {
    use crate::app;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::util::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn ipv4_dest() {
        for (from, key, to) in [
            ("10.0.0.0", "1.2.3.255", "11.2.3.255"),
            ("128.128.33.0", "255.0.255.33", "127.128.32.33"),
        ] {
            let app = app();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/2/dest?from={from}&key={key}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(body, to);
        }
    }

    #[tokio::test]
    async fn ipv4_key() {
        for (from, to, key) in [
            ("10.0.0.0", "11.2.3.255", "1.2.3.255"),
            ("128.128.33.0", "127.128.32.33", "255.0.255.33"),
        ] {
            let app = app();

            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/2/key?from={from}&to={to}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(body, key);
        }
    }

    #[tokio::test]
    async fn ipv6_dest() {
        let (from, key, to) = ("fe80::1", "5:6:7::3333", "fe85:6:7::3332");

        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/2/v6/dest?from={from}&key={key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, to);
    }

    #[tokio::test]
    async fn ipv6_key() {
        let (from, to, key) = (
            "aaaa::aaaa",
            "5555:ffff:c:0:0:c:1234:5555",
            "ffff:ffff:c::c:1234:ffff",
        );

        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/2/v6/key?from={from}&to={to}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, key);
    }
}
