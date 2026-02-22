use anyhow::Error;
use serde_json::Value;

use crate::TransactionEvent;

pub fn normalize_paystack_payload(raw: &str) -> Result<TransactionEvent, Error> {
    let json: Value = serde_json::from_str(raw)?;

    let data = json.get("data").unwrap_or(&json);

    let transaction_ref = data["reference"]
        .as_str()
        .or_else(|| data["data"]["reference"].as_str())
        .unwrap_or("")
        .to_string();

    let user_id = data["customer"]["id"]
        .as_str()
        .or_else(|| data["customer"]["email"].as_str())
        .unwrap_or("")
        .to_string();

    let application_id = data["meta"]["application_id"]
        .as_str()
        .or_else(|| data["channel"].as_str())
        .unwrap_or("paystack")
        .to_string();

    let status = data["status"].as_str().unwrap_or("").to_string();

    let amount = data["amount"]
        .as_f64()
        .or_else(|| data["amount"].as_i64().map(|v| v as f64))
        .unwrap_or(0.0);

    let category = data["gateway_response"]
        .as_str()
        .unwrap_or("PAYSTACK")
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
    fn normalize_paystack_basic_payload() {
        let raw = r#"
        {
          "event": "charge.success",
          "data": {
            "reference": "psk-123",
            "status": "success",
            "amount": 250000,
            "channel": "card",
            "gateway_response": "Approved",
            "customer": {
              "id": "cust-psk-1",
              "email": "user@example.com"
            },
            "meta": {
              "application_id": "ourpocket-app"
            }
          }
        }
        "#;

        let event =
            normalize_paystack_payload(raw).expect("normalize_paystack_payload should succeed");

        assert_eq!(event.transaction_ref, "psk-123");
        assert_eq!(event.user_id, "cust-psk-1");
        assert_eq!(event.application_id, "ourpocket-app");
        assert_eq!(event.status, "success");
        assert_eq!(event.amount, 250000.0);
        assert_eq!(event.category, "Approved");
    }
}
