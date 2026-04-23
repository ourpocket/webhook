use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use domain::normalize_webhook_payload;
use queue::{EventQueue, RedisQueue};
use std::env;

#[derive(Clone)]
struct AppState<Q: EventQueue + Clone + 'static> {
    queue: Q,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let queue_key =
        env::var("REDIS_QUEUE_KEY").unwrap_or_else(|_| "ourpocket:transactions".to_string());

    let queue = RedisQueue::new(&redis_url, queue_key).await?;
    let state = AppState { queue };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/webhook/:provider", post(webhook_handler::<RedisQueue>))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> StatusCode {
    StatusCode::OK
}

async fn webhook_handler<Q>(
    Path(provider): Path<String>,
    State(state): State<AppState<Q>>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, StatusCode>
where
    Q: EventQueue + Clone + 'static,
{
    // Verify webhook signature based on provider
    let signature = extract_signature(&provider, &headers);

    if !verify_signature(&provider, &body, signature) {
        eprintln!("Invalid webhook signature for provider: {}", provider);
        return Err(StatusCode::UNAUTHORIZED);
    }

    let event = normalize_webhook_payload(&body, &provider).map_err(|_| StatusCode::BAD_REQUEST)?;

    println!(
        "Received webhook from {} for ref: {}",
        provider, event.transaction_ref
    );

    state
        .queue
        .publish(event)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

fn extract_signature<'a>(provider: &str, headers: &'a HeaderMap) -> &'a str {
    match provider {
        "paystack" => headers
            .get("x-paystack-signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
        "flutterwave" => headers
            .get("verif-hash")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
        _ => headers
            .get("x-signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    }
}

fn verify_signature(provider: &str, body: &str, signature: &str) -> bool {
    match provider {
        "flutterwave" => {
            let secret = env::var("FLUTTERWAVE_WEBHOOK_SECRET").unwrap_or_default();
            verify_hmac(body, signature, &secret)
        }
        "paystack" => {
            let secret = env::var("PAYSTACK_WEBHOOK_SECRET").unwrap_or_default();
            verify_hmac(body, signature, &secret)
        }
        _ => {
            // In dev/testing, allow unknown providers
            env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()) != "production"
        }
    }
}

fn verify_hmac(body: &str, signature: &str, secret: &str) -> bool {
    if secret.is_empty() {
        // Skip verification if no secret configured (dev mode)
        return true;
    }

    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(body.as_bytes());

    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());

    // Timing-safe comparison
    use subtle::ConstantTimeEq;
    expected.as_bytes().ct_eq(signature.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use queue::InMemoryQueue;
    use tower::ServiceExt;

    #[tokio::test]
    async fn webhook_handler_returns_ok_for_flutterwave_payload() {
        let queue = InMemoryQueue::new(16);
        let state = AppState { queue };

        let app = Router::new()
            .route("/webhook/:provider", post(webhook_handler::<InMemoryQueue>))
            .with_state(state);

        let body = r#"
        {
            "provider": "flutterwave",
            "data": {
                "tx_ref": "abc123",
                "status": "SUCCESS",
                "amount": 100.5,
                "payment_type": "PAYMENT",
                "customer": {
                    "id": "user1"
                },
                "meta": {
                    "application_id": "app1"
                }
            }
        }
        "#;

        let request = Request::builder()
            .method("POST")
            .uri("/webhook/flutterwave")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
