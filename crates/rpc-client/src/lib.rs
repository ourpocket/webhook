use anyhow::Result;
use domain::TransactionEvent;
use serde::Serialize;

#[derive(Clone)]
pub struct AppConfig {
    pub environment: Environment,
}

#[derive(Clone, Copy)]
pub enum Environment {
    Staging,
    Production,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "staging".to_string());
        let environment = match env.to_lowercase().as_str() {
            "production" | "prod" => Environment::Production,
            _ => Environment::Staging,
        };

        Self { environment }
    }

    pub fn backend_url(&self) -> String {
        match self.environment {
            Environment::Staging => std::env::var("BACKEND_STAGING_URL")
                .unwrap_or_else(|_| "https://staging.ourpocket.com".to_string()),
            Environment::Production => std::env::var("BACKEND_PROD_URL")
                .unwrap_or_else(|_| "https://api.ourpocket.com".to_string()),
        }
    }
}

#[async_trait::async_trait]
pub trait BackendClient {
    async fn send_transaction(&self, event: TransactionEvent) -> Result<()>;
}

#[derive(Clone)]
pub struct HttpBackendClient {
    http_client: reqwest::Client,
    config: AppConfig,
}

impl HttpBackendClient {
    pub fn new(config: AppConfig) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            config,
        }
    }
}

#[derive(Serialize)]
struct TransactionRequestDto {
    transaction_ref: String,
    user_id: String,
    application_id: String,
    status: String,
    amount: f64,
    category: String,
    metadata: serde_json::Value,
}

#[async_trait::async_trait]
impl BackendClient for HttpBackendClient {
    async fn send_transaction(&self, event: TransactionEvent) -> Result<()> {
        let url = format!("{}/transactions/webhook", self.config.backend_url());

        let payload = TransactionRequestDto {
            transaction_ref: event.transaction_ref,
            user_id: event.user_id,
            application_id: event.application_id,
            status: event.status,
            amount: event.amount,
            category: event.category,
            metadata: event.metadata,
        };

        self.http_client.post(url).json(&payload).send().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_config_selects_environment_from_env() {
        unsafe { std::env::set_var("APP_ENV", "staging") };
        let cfg = AppConfig::from_env();
        assert!(matches!(cfg.environment, Environment::Staging));

        unsafe { std::env::set_var("APP_ENV", "production") };
        let cfg = AppConfig::from_env();
        assert!(matches!(cfg.environment, Environment::Production));
    }

    #[test]
    fn backend_url_reads_from_env_with_fallbacks() {
        unsafe {
            std::env::remove_var("BACKEND_STAGING_URL");
            std::env::remove_var("BACKEND_PROD_URL");
        }

        let staging = AppConfig {
            environment: Environment::Staging,
        };
        assert!(staging.backend_url().contains("staging.ourpocket.com"));

        let prod = AppConfig {
            environment: Environment::Production,
        };
        assert!(prod.backend_url().contains("api.ourpocket.com"));

        unsafe {
            std::env::set_var("BACKEND_STAGING_URL", "https://example-staging");
            std::env::set_var("BACKEND_PROD_URL", "https://example-prod");
        }

        assert_eq!(staging.backend_url(), "https://example-staging".to_string());
        assert_eq!(prod.backend_url(), "https://example-prod".to_string());
    }
}
