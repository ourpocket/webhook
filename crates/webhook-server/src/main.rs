use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use domain::normalize_payload;
use queue::{EventQueue, InMemoryQueue};

#[derive(Clone)]
struct AppState<Q: EventQueue + Clone + 'static> {
    queue: Q,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let queue = InMemoryQueue::new(1024);
    let state = AppState { queue };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/webhook", post(webhook_handler::<InMemoryQueue>))
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
    let event = normalize_payload(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

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
    async fn webhook_handler_returns_ok_for_valid_payload() {
        let queue = InMemoryQueue::new(16);
        let state = AppState { queue };

        let app = Router::new()
            .route("/webhook", post(webhook_handler::<InMemoryQueue>))
            .with_state(state);

        let body = r#"
        {
            "transactionRef": "abc123",
            "userId": "user1",
            "applicationId": "app1",
            "status": "SUCCESS",
            "amount": 100.5,
            "category": "PAYMENT"
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
