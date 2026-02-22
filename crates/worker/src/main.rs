use anyhow::Result;
use dotenvy::dotenv;
use queue::{EventQueueConsumer, InMemoryQueue};
use rpc_client::{AppConfig, BackendClient, HttpBackendClient};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // In a real deployment, this would connect to an external queue like SQS/Kafka/NATS.
    // For now, this uses an in-memory queue instance as a placeholder.
    let queue = InMemoryQueue::new(1024);

    let config = AppConfig::from_env();
    let backend_client = HttpBackendClient::new(config);

    worker_loop(queue, backend_client).await
}

async fn worker_loop<Q, C>(queue: Q, backend_client: C) -> Result<()>
where
    Q: EventQueueConsumer,
    C: BackendClient,
{
    loop {
        let event = queue.consume().await?;
        backend_client.send_transaction(event).await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::TransactionEvent;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockQueue {
        events: Arc<Mutex<Vec<TransactionEvent>>>,
    }

    #[async_trait]
    impl EventQueueConsumer for MockQueue {
        async fn consume(&self) -> Result<TransactionEvent> {
            let mut guard = self.events.lock().await;
            guard
                .pop()
                .ok_or_else(|| anyhow::Error::msg("no events left"))
        }
    }

    struct MockBackend {
        calls: Arc<Mutex<Vec<TransactionEvent>>>,
    }

    #[async_trait]
    impl BackendClient for MockBackend {
        async fn send_transaction(&self, event: TransactionEvent) -> Result<()> {
            let mut guard = self.calls.lock().await;
            guard.push(event);
            Ok(())
        }
    }

    #[tokio::test]
    async fn worker_loop_processes_events_until_error() {
        let events = Arc::new(Mutex::new(vec![TransactionEvent {
            transaction_ref: "ref-1".to_string(),
            user_id: "user-1".to_string(),
            application_id: "app".to_string(),
            status: "SUCCESS".to_string(),
            amount: 1.0,
            category: "TEST".to_string(),
            metadata: serde_json::json!({}),
        }]));

        let calls = Arc::new(Mutex::new(Vec::new()));

        let queue = MockQueue {
            events: Arc::clone(&events),
        };
        let backend = MockBackend {
            calls: Arc::clone(&calls),
        };

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            worker_loop(queue, backend),
        )
        .await;

        assert!(result.is_err() || result.unwrap().is_err());

        let recorded = calls.lock().await;
        assert_eq!(recorded.len(), 1);
    }
}
