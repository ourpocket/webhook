// ============================================================================
// MNO (Mobile Network Operator) Webhook Normalizers
// ============================================================================
// Handles MTN MoMo, Airtel Money, Orange Money webhooks

use crate::TransactionEvent;
use anyhow::Error;
use serde_json::Value;

pub fn normalize_mtn_momo_payload(raw: &str) -> Result<TransactionEvent, Error> {
    let json: Value = serde_json::from_str(raw)?;

    let transaction_ref = json["referenceId"]
        .as_str()
        .or_else(|| json["externalId"].as_str())
        .unwrap_or("")
        .to_string();

    let user_id = json["payer"]["partyId"].as_str().unwrap_or("").to_string();

    let status = match json["status"].as_str() {
        Some("SUCCESSFUL") => "success",
        Some("PENDING") => "pending",
        Some("FAILED") => "failed",
        _ => "unknown",
    }
    .to_string();

    let amount = json["amount"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| json["amount"].as_f64())
        .unwrap_or(0.0);

    Ok(TransactionEvent {
        transaction_ref,
        user_id,
        application_id: "mtn-momo".to_string(),
        status,
        amount,
        category: "mobile_money".to_string(),
        metadata: json,
    })
}

pub fn normalize_airtel_money_payload(raw: &str) -> Result<TransactionEvent, Error> {
    let json: Value = serde_json::from_str(raw)?;

    let transaction_ref = json["transaction"]["id"].as_str().unwrap_or("").to_string();

    let user_id = json["subscriber"]["msisdn"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let status = match json["transaction"]["status"].as_str() {
        Some("TS") => "success",
        Some("TF") => "failed",
        Some("TA") => "cancelled",
        _ => "pending",
    }
    .to_string();

    let amount = json["transaction"]["amount"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Ok(TransactionEvent {
        transaction_ref,
        user_id,
        application_id: "airtel-money".to_string(),
        status,
        amount,
        category: "mobile_money".to_string(),
        metadata: json,
    })
}

pub fn normalize_orange_money_payload(raw: &str) -> Result<TransactionEvent, Error> {
    let json: Value = serde_json::from_str(raw)?;

    let transaction_ref = json["transaction_id"]
        .as_str()
        .or_else(|| json["reference_number"].as_str())
        .unwrap_or("")
        .to_string();

    let user_id = json["msisdn"].as_str().unwrap_or("").to_string();

    let status = match json["status"].as_str() {
        Some("SUCCESS") => "success",
        Some("PENDING") => "pending",
        Some("FAILED") => "failed",
        Some("CANCELLED") => "cancelled",
        _ => "unknown",
    }
    .to_string();

    let amount = json["amount"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Ok(TransactionEvent {
        transaction_ref,
        user_id,
        application_id: "orange-money".to_string(),
        status,
        amount,
        category: "mobile_money".to_string(),
        metadata: json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_mtn_momo_basic_payload() {
        let raw = r#"
        {
            "referenceId": "mtn-ref-123",
            "externalId": "ext-456",
            "status": "SUCCESSFUL",
            "amount": "500",
            "payer": {
                "partyId": "+256771234567",
                "partyIdType": "MSISDN"
            },
            "currency": "UGX"
        }
        "#;

        let event = normalize_mtn_momo_payload(raw).expect("should succeed");
        assert_eq!(event.transaction_ref, "mtn-ref-123");
        assert_eq!(event.user_id, "+256771234567");
        assert_eq!(event.status, "success");
        assert_eq!(event.application_id, "mtn-momo");
        assert_eq!(event.category, "mobile_money");
    }

    #[test]
    fn normalize_airtel_money_basic_payload() {
        let raw = r#"
        {
            "transaction": {
                "id": "airtel-tx-789",
                "status": "TS",
                "amount": "1000"
            },
            "subscriber": {
                "msisdn": "+254712345678",
                "country": "KE",
                "currency": "KES"
            }
        }
        "#;

        let event = normalize_airtel_money_payload(raw).expect("should succeed");
        assert_eq!(event.transaction_ref, "airtel-tx-789");
        assert_eq!(event.user_id, "+254712345678");
        assert_eq!(event.status, "success");
        assert_eq!(event.application_id, "airtel-money");
    }

    #[test]
    fn normalize_orange_money_basic_payload() {
        let raw = r#"
        {
            "transaction_id": "orange-tx-001",
            "reference_number": "ref-002",
            "status": "SUCCESS",
            "amount": "5000",
            "msisdn": "+22501234567",
            "currency": "XOF"
        }
        "#;

        let event = normalize_orange_money_payload(raw).expect("should succeed");
        assert_eq!(event.transaction_ref, "orange-tx-001");
        assert_eq!(event.user_id, "+22501234567");
        assert_eq!(event.status, "success");
        assert_eq!(event.application_id, "orange-money");
    }
}
