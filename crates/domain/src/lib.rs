mod flutterwave;
mod mno;
mod paystack;
mod transaction;

pub use flutterwave::normalize_flutterwave_payload;
pub use mno::{
    normalize_airtel_money_payload, normalize_mtn_momo_payload, normalize_orange_money_payload,
};
pub use paystack::normalize_paystack_payload;
pub use transaction::{TransactionEvent, normalize_payload};

pub fn normalize_webhook_payload(raw: &str) -> Result<TransactionEvent, anyhow::Error> {
    let json: serde_json::Value = serde_json::from_str(raw)?;
    let provider = json["provider"].as_str().unwrap_or("").to_lowercase();

    match provider.as_str() {
        "flutterwave" => normalize_flutterwave_payload(raw),
        "paystack" => normalize_paystack_payload(raw),
        "mtn-momo" => normalize_mtn_momo_payload(raw),
        "airtel-money" => normalize_airtel_money_payload(raw),
        "orange-money" => normalize_orange_money_payload(raw),
        _ => Err(anyhow::Error::msg("unsupported provider")),
    }
}
