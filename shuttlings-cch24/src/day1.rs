use axum::response::{IntoResponse, Response};
use http::{header::LOCATION, StatusCode};

pub async fn seek() -> Response {
    let uri = "https://www.youtube.com/watch?v=9Gc4QTqslN4";
    (StatusCode::FOUND, [(LOCATION, uri)]).into_response()
}

#[cfg(test)]
mod day1_tests {
    use crate::app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::util::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn seek() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/-1/seek")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);

        let (name, val) = response.headers().iter().next().unwrap();
        assert_eq!(name, "location");
        assert_eq!(val, "https://www.youtube.com/watch?v=9Gc4QTqslN4");
    }
}

#[cfg(test)]
mod tests {
    use crate::app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::util::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn seek() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/-1/seek")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);

        let (name, val) = response.headers().iter().next().unwrap();
        assert_eq!(name, "location");
        assert_eq!(val, "https://www.youtube.com/watch?v=9Gc4QTqslN4");
    }
}
