use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use domain::normalize_webhook_payload;
use queue::{EventQueue, RedisQueue};

#[derive(Clone)]
struct AppState<Q: EventQueue + Clone + 'static> {
    queue: Q,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let queue_key =
        std::env::var("REDIS_QUEUE_KEY").unwrap_or_else(|_| "ourpocket:transactions".to_string());

    let queue = RedisQueue::new(&redis_url, queue_key)?;
    let state = AppState { queue };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/webhook", post(webhook_handler::<RedisQueue>))
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
    State(state): State<AppState<Q>>,
    body: String,
) -> Result<StatusCode, StatusCode>
where
    Q: EventQueue + Clone + 'static,
{
    let event = normalize_webhook_payload(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    state
        .queue
        .publish(event)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
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
            .route("/webhook", post(webhook_handler::<InMemoryQueue>))
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
            .uri("/webhook")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
