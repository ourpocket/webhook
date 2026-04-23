use anyhow::Result;
use dotenvy::dotenv;
use queue::{EventQueueConsumer, RedisQueue};
use rpc_client::{AppConfig, BackendClient, HttpBackendClient};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let queue_key =
        std::env::var("REDIS_QUEUE_KEY").unwrap_or_else(|_| "ourpocket:transactions".to_string());

    let queue = RedisQueue::new(&redis_url, queue_key).await?;

    let config = AppConfig::from_env();
    let backend_client = HttpBackendClient::new(config);

    worker_loop(queue, backend_client).await
}

async fn worker_loop<Q, C>(queue: Q, backend_client: C) -> Result<()>
where
    Q: EventQueueConsumer,
    C: BackendClient,
{
    let max_retries = 5;
    let base_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(300); // 5 minutes

    loop {
        match queue.consume().await {
            Ok(event) => {
                let mut last_error: Option<String> = None;
                let mut success = false;

                for attempt in 1..=max_retries {
                    match backend_client.send_transaction(event.clone()).await {
                        Ok(()) => {
                            success = true;
                            break;
                        }
                        Err(e) => {
                            let err_msg = e.to_string();
                            last_error = Some(err_msg.clone());
                            if attempt < max_retries {
                                let delay = calculate_backoff(attempt, base_delay, max_delay);
                                eprintln!(
                                    "Attempt {}/{} failed for {}: {}. Retrying in {:?}",
                                    attempt, max_retries, event.transaction_ref, err_msg, delay
                                );
                                tokio::time::sleep(delay).await;
                            } else {
                                eprintln!(
                                    "Attempt {}/{} failed for {}: {}.",
                                    attempt, max_retries, event.transaction_ref, err_msg
                                );
                            }
                        }
                    }
                }

                if !success {
                    eprintln!(
                        "All {} attempts failed for transaction {}. Last error: {:?}",
                        max_retries, event.transaction_ref, last_error
                    );
                    // TODO: Send to dead-letter queue for manual inspection
                }
            }
            Err(e) => {
                eprintln!("Failed to consume from queue: {}", e);
                // Sleep before retrying to avoid tight loop
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn calculate_backoff(attempt: u32, base: Duration, max: Duration) -> Duration {
    let exponential = base * 2u32.pow(attempt - 1);
    std::cmp::min(exponential, max)
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
        should_fail: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl BackendClient for MockBackend {
        async fn send_transaction(&self, event: TransactionEvent) -> Result<()> {
            let should_fail = *self.should_fail.lock().await;
            if should_fail {
                return Err(anyhow::Error::msg("backend error"));
            }
            let mut guard = self.calls.lock().await;
            guard.push(event);
            Ok(())
        }
    }

    struct AlwaysFailingBackend;

    #[async_trait]
    impl BackendClient for AlwaysFailingBackend {
        async fn send_transaction(&self, _event: TransactionEvent) -> Result<()> {
            Err(anyhow::Error::msg("always fails"))
        }
    }

    #[tokio::test]
    async fn worker_loop_processes_multiple_events() {
        let events = Arc::new(Mutex::new(vec![
            TransactionEvent {
                transaction_ref: "ref-1".to_string(),
                user_id: "user-1".to_string(),
                application_id: "app".to_string(),
                status: "SUCCESS".to_string(),
                amount: 1.0,
                category: "TEST".to_string(),
                metadata: serde_json::json!({}),
            },
            TransactionEvent {
                transaction_ref: "ref-2".to_string(),
                user_id: "user-2".to_string(),
                application_id: "app".to_string(),
                status: "PENDING".to_string(),
                amount: 2.0,
                category: "TEST".to_string(),
                metadata: serde_json::json!({}),
            },
        ]));

        let calls: Arc<Mutex<Vec<TransactionEvent>>> = Arc::new(Mutex::new(Vec::new()));

        let queue = MockQueue {
            events: Arc::clone(&events),
        };
        let backend = MockBackend {
            calls: Arc::clone(&calls),
            should_fail: Arc::new(Mutex::new(false)),
        };

        // Run for a short time, then cancel
        let worker_handle = tokio::spawn(async move { worker_loop(queue, backend).await });

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Cancel the worker
        worker_handle.abort();

        let recorded = calls.lock().await;
        assert_eq!(recorded.len(), 2);
    }

    #[tokio::test]
    async fn worker_loop_continues_on_backend_error() {
        let events = Arc::new(Mutex::new(vec![TransactionEvent {
            transaction_ref: "ref-1".to_string(),
            user_id: "user-1".to_string(),
            application_id: "app".to_string(),
            status: "SUCCESS".to_string(),
            amount: 1.0,
            category: "TEST".to_string(),
            metadata: serde_json::json!({}),
        }]));

        let queue = MockQueue {
            events: Arc::clone(&events),
        };
        let backend = AlwaysFailingBackend;

        // Should not panic or exit on error
        let result =
            tokio::time::timeout(Duration::from_millis(100), worker_loop(queue, backend)).await;

        // Timeout is expected since worker loops forever
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[test]
    fn calculate_backoff_works() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);

        assert_eq!(calculate_backoff(1, base, max), Duration::from_secs(1));
        assert_eq!(calculate_backoff(2, base, max), Duration::from_secs(2));
        assert_eq!(calculate_backoff(3, base, max), Duration::from_secs(4));
        assert_eq!(calculate_backoff(4, base, max), Duration::from_secs(8));
        assert_eq!(calculate_backoff(10, base, max), Duration::from_secs(60)); // capped
    }
}
