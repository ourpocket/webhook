use std::sync::Arc;

use anyhow::Error;
use async_trait::async_trait;
use domain::TransactionEvent;
use tokio::sync::mpsc;

#[async_trait]
pub trait EventQueue: Send + Sync {
    async fn publish(&self, event: TransactionEvent) -> Result<(), Error>;
}

#[async_trait]
pub trait EventQueueConsumer: Send + Sync {
    async fn consume(&self) -> Result<TransactionEvent, Error>;
}

#[derive(Clone)]
pub struct InMemoryQueue {
    sender: mpsc::Sender<TransactionEvent>,
    receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<TransactionEvent>>>,
}

impl InMemoryQueue {
    pub fn new(buffer: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer);
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }
}

#[async_trait]
impl EventQueue for InMemoryQueue {
    async fn publish(&self, event: TransactionEvent) -> Result<(), Error> {
        self.sender
            .send(event)
            .await
            .map_err(|err| Error::msg(err.to_string()))
    }
}

#[async_trait]
impl EventQueueConsumer for InMemoryQueue {
    async fn consume(&self) -> Result<TransactionEvent, Error> {
        let mut rx = self.receiver.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| Error::msg("queue closed".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_event(id: &str) -> TransactionEvent {
        TransactionEvent {
            transaction_ref: format!("ref-{id}"),
            user_id: id.to_string(),
            application_id: "app".to_string(),
            status: "SUCCESS".to_string(),
            amount: 10.0,
            category: "TEST".to_string(),
            metadata: serde_json::json!({ "id": id }),
        }
    }

    #[tokio::test]
    async fn publish_and_consume_round_trip() {
        let queue = InMemoryQueue::new(8);

        let event = build_event("user-1");
        queue
            .publish(event.clone())
            .await
            .expect("publish should succeed");

        let consumed = queue.consume().await.expect("consume should succeed");
        assert_eq!(consumed, event);
    }

    #[tokio::test]
    async fn consume_returns_error_when_queue_closed() {
        let (sender, receiver) = mpsc::channel(1);
        drop(sender);

        let queue = InMemoryQueue {
            sender: mpsc::channel(1).0,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        };

        let result = queue.consume().await;
        assert!(result.is_err());
    }
}
