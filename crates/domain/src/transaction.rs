use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TransactionEvent {
    pub transaction_ref: String,
    pub user_id: String,
    pub application_id: String,
    pub status: String,
    pub amount: f64,
    pub category: String,
    pub metadata: serde_json::Value,
}

pub fn normalize_payload(raw: &str) -> Result<TransactionEvent, anyhow::Error> {
    let json: serde_json::Value = serde_json::from_str(raw)?;

    Ok(TransactionEvent {
        transaction_ref: json["transactionRef"].as_str().unwrap_or("").to_string(),
        user_id: json["userId"].as_str().unwrap_or("").to_string(),
        application_id: json["applicationId"].as_str().unwrap_or("").to_string(),
        status: json["status"].as_str().unwrap_or("").to_string(),
        amount: json["amount"].as_f64().unwrap_or(0.0),
        category: json["category"].as_str().unwrap_or("").to_string(),
        metadata: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_payload_basic_fields() {
        let raw = r#"
        {
            "transactionRef": "abc123",
            "userId": "user1",
            "applicationId": "app1",
            "status": "SUCCESS",
            "amount": 100.5,
            "category": "PAYMENT"
        }
        "#;

        let event = normalize_payload(raw).expect("normalize_payload should succeed");

        assert_eq!(
            event,
            TransactionEvent {
                transaction_ref: "abc123".to_string(),
                user_id: "user1".to_string(),
                application_id: "app1".to_string(),
                status: "SUCCESS".to_string(),
                amount: 100.5,
                category: "PAYMENT".to_string(),
                metadata: serde_json::from_str(raw).unwrap()
            }
        );
    }
}
