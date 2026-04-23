use anyhow::Error;
use serde_json::Value;

use crate::TransactionEvent;

pub fn normalize_flutterwave_payload(raw: &str) -> Result<TransactionEvent, Error> {
    let json: Value = serde_json::from_str(raw)?;

    let data = json.get("data").unwrap_or(&json);

    let transaction_ref = data["tx_ref"]
        .as_str()
        .or_else(|| data["txRef"].as_str())
        .unwrap_or("")
        .to_string();

    // Flutterwave customer.id is numeric, handle both int and string
    let user_id = data["customer"]["id"]
        .as_u64()
        .map(|id| id.to_string())
        .or_else(|| data["customer"]["id"].as_str().map(String::from))
        .or_else(|| data["customer"]["email"].as_str().map(String::from))
        .unwrap_or_default();

    let application_id = data["app_id"]
        .as_str()
        .or_else(|| data["meta"]["application_id"].as_str())
        .unwrap_or("flutterwave")
        .to_string();

    let status = data["status"].as_str().unwrap_or("").to_string();

    let amount = data["amount"]
        .as_f64()
        .or_else(|| data["amount"].as_i64().map(|v| v as f64))
        .unwrap_or(0.0);

    let category = data["payment_type"]
        .as_str()
        .unwrap_or("FLUTTERWAVE")
        .to_string();

    Ok(TransactionEvent {
        transaction_ref,
        user_id,
        application_id,
        status,
        amount,
        category,
        metadata: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_flutterwave_basic_payload() {
        let raw = r#"
        {
          "data": {
            "tx_ref": "flw-123",
            "status": "successful",
            "amount": 5000,
            "payment_type": "card",
            "customer": {
              "id": "cust-1",
              "email": "test@example.com"
            },
            "meta": {
              "application_id": "ourpocket-app"
            }
          }
        }
        "#;

        let event = normalize_flutterwave_payload(raw)
            .expect("normalize_flutterwave_payload should succeed");

        assert_eq!(event.transaction_ref, "flw-123");
        assert_eq!(event.user_id, "cust-1");
        assert_eq!(event.application_id, "ourpocket-app");
        assert_eq!(event.status, "successful");
        assert_eq!(event.amount, 5000.0);
        assert_eq!(event.category, "card");
    }
}
