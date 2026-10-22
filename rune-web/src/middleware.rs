use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::StatusCode,
    http::header::{HOST, ORIGIN},
};

pub async fn validate_host(req: Request, next: Next) -> Result<Response, StatusCode> {
    if let Some(host) = req.headers().get(HOST) {
        if let Ok(host_str) = host.to_str() {
            // Check if it's localhost or 127.0.0.1
            // We strip port if present for simpler comparison, or just check prefixes
            let valid_hosts = ["localhost", "127.0.0.1"];
            
            let host_without_port = host_str.split(':').next().unwrap_or(host_str);
            
            if !valid_hosts.contains(&host_without_port) {
                return Err(StatusCode::FORBIDDEN);
            }
        } else {
            return Err(StatusCode::BAD_REQUEST);
        }
    } else {
        // Require Host header
        return Err(StatusCode::BAD_REQUEST);
    }
    
    if let Some(origin) = req.headers().get(ORIGIN) {
        if let Ok(origin_str) = origin.to_str() {
            // Origin typically looks like "http://127.0.0.1:8080"
            let is_valid = origin_str.starts_with("http://127.0.0.1") || 
                           origin_str.starts_with("http://localhost") ||
                           origin_str.starts_with("https://127.0.0.1") ||
                           origin_str.starts_with("https://localhost");
            
            if !is_valid {
                return Err(StatusCode::FORBIDDEN);
            }
        } else {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_valid_host() {
        let app = Router::new()
            .route("/", get(|| async { "Hello" }))
            .layer(axum::middleware::from_fn(validate_host));

        let req = HttpRequest::builder()
            .uri("/")
            .header(HOST, "127.0.0.1:8080")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_host() {
        let app = Router::new()
            .route("/", get(|| async { "Hello" }))
            .layer(axum::middleware::from_fn(validate_host));

        let req = HttpRequest::builder()
            .uri("/")
            .header(HOST, "malicious.com:8080")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_valid_origin() {
        let app = Router::new()
            .route("/", get(|| async { "Hello" }))
            .layer(axum::middleware::from_fn(validate_host));

        let req = HttpRequest::builder()
            .uri("/")
            .header(HOST, "localhost:8080")
            .header(ORIGIN, "http://localhost:8080")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_origin() {
        let app = Router::new()
            .route("/", get(|| async { "Hello" }))
            .layer(axum::middleware::from_fn(validate_host));

        let req = HttpRequest::builder()
            .uri("/")
            .header(HOST, "localhost:8080")
            .header(ORIGIN, "http://attacker.com")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }
}
