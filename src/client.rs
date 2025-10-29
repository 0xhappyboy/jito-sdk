use crate::global::BUNDLE_RPC;
use crate::global::TRANSACTIONS_POOL_RPC;
use base64::{Engine, prelude::BASE64_STANDARD};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};

pub enum ClientEnum {
    Bundle,
    Tip,
    BlockEngine,
    Validators,
    TransactionsPool,
    Health,
    Statistics,
}

impl ClientEnum {}

/// ============== bundle client ==============

#[derive(Debug, Clone)]
pub struct BundleClient {
    client: Client,
}

#[derive(Debug)]
pub enum BundleError {
    RequestFailed(String),
    SerializationError(String),
    BundleRejected(String),
    ApiError(String),
}

#[derive(Debug, Serialize)]
struct BundleRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<BundleParams>,
}

#[derive(Debug, Serialize)]
struct BundleParams {
    txs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tip_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tip_amount: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct BundleResponse {
    result: Option<BundleResult>,
    error: Option<JitoError>,
}

#[derive(Debug, Deserialize)]
struct BundleResult {
    bundle_id: String,
}

#[derive(Debug, Deserialize)]
struct JitoError {
    message: String,
}

impl BundleClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn send_bundle(
        &self,
        transactions: Vec<Transaction>,
        tip_account: Option<Pubkey>,
        tip_amount: Option<u64>,
    ) -> Result<String, BundleError> {
        let encoded_txs: Vec<String> = transactions
            .into_iter()
            .map(|tx| {
                // Serialize the entire transaction (signature + message).
                let mut serialized = Vec::new();
                // Number of serialized signatures
                serialized.extend_from_slice(&(tx.signatures.len() as u64).to_le_bytes());
                // Serialize all signatures
                for signature in &tx.signatures {
                    serialized.extend_from_slice(signature.as_ref());
                }
                // Serialization message data
                let message_data = tx.message_data();
                serialized.extend_from_slice(&message_data);
                Ok(BASE64_STANDARD.encode(&serialized))
            })
            .collect::<Result<Vec<_>, BundleError>>()?;
        let params = BundleParams {
            txs: encoded_txs,
            tip_account: tip_account.map(|pk| pk.to_string()),
            tip_amount,
        };
        let request = BundleRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "sendBundle".to_string(),
            params: vec![params],
        };
        let response = self
            .client
            .post(BUNDLE_RPC)
            .json(&request)
            .send()
            .await
            .map_err(|e| BundleError::ApiError(format!("{:?}", e)))?;
        self.handle_response(response).await
    }

    pub async fn get_bundle_status(&self, bundle_id: &str) -> Result<BundleStatus, BundleError> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBundleStatus",
            "params": [bundle_id]
        });
        let response = self
            .client
            .post(BUNDLE_RPC)
            .json(&request)
            .send()
            .await
            .map_err(|e| BundleError::ApiError(format!("{:?}", e)))?;

        let status_response: BundleStatusResponse = response
            .json()
            .await
            .map_err(|e| BundleError::ApiError(e.to_string()))?;
        Ok(status_response.result)
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<String, BundleError> {
        if !response.status().is_success() {
            return Err(BundleError::ApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response
                    .text()
                    .await
                    .map_err(|e| BundleError::ApiError(format!("{:?}", e)))?
            )));
        }
        let bundle_response: BundleResponse = response
            .json()
            .await
            .map_err(|e| BundleError::ApiError(format!("{:?}", e)))?;

        if let Some(error) = bundle_response.error {
            return Err(BundleError::BundleRejected(error.message));
        }
        bundle_response
            .result
            .map(|r| r.bundle_id)
            .ok_or_else(|| BundleError::ApiError("No result in response".to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct BundleStatusResponse {
    result: BundleStatus,
}

#[derive(Debug, Deserialize)]
pub struct BundleStatus {
    pub bundle_id: String,
    pub status: String,
    pub slot: Option<u64>,
}

/// ============== tip client ==============
use crate::global::TIP_RPC;

#[derive(Debug, Clone)]
pub struct TipClient {
    client: Client,
}

#[derive(Debug)]
pub enum TipError {
    RequestFailed(String),
    ApiError(String),
    NoTipAccounts,
}

#[derive(Debug, Deserialize)]
struct TipAccountsResponse {
    tip_accounts: Vec<TipAccount>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TipAccount {
    pub pubkey: String,
    pub lamports_per_signature: u64,
}

impl TipClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    pub async fn get_tip_accounts(&self) -> Result<Vec<TipAccount>, TipError> {
        let response = self
            .client
            .get(TIP_RPC)
            .send()
            .await
            .map_err(|e| TipError::ApiError(format!("{:?}", e)))?;
        if !response.status().is_success() {
            return Err(TipError::ApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response
                    .text()
                    .await
                    .map_err(|e| TipError::ApiError(format!("{:?}", e)))?
            )));
        }
        let tip_response: TipAccountsResponse = response
            .json()
            .await
            .map_err(|e| TipError::ApiError(format!("{:?}", e)))?;
        Ok(tip_response.tip_accounts)
    }
    pub async fn get_optimal_tip_account(&self) -> Result<TipAccount, TipError> {
        let tip_accounts = self.get_tip_accounts().await?;
        tip_accounts
            .into_iter()
            .max_by_key(|account| account.lamports_per_signature)
            .ok_or(TipError::NoTipAccounts)
    }
    pub async fn get_recommended_tip(&self) -> Result<u64, TipError> {
        let optimal_account = self.get_optimal_tip_account().await?;
        Ok(optimal_account.lamports_per_signature)
    }
}

/// ============== Block Engine Client ==============
use crate::global::BLOCK_EGNINE_RPC;

#[derive(Debug, Clone)]
pub struct BlockEngineClient {
    client: Client,
}

#[derive(Debug)]
pub enum BlockEngineError {
    RequestFailed(String),
    ApiError(String),
}

#[derive(Debug, Deserialize)]
pub struct BlockEngineResponse {
    pub leaders: Vec<Leader>,
    pub congestion: f64,
    pub current_slot: u64,
}

#[derive(Debug, Deserialize)]
pub struct Leader {
    pub pubkey: String,
    pub slot: u64,
}

impl BlockEngineClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_block_engine_info(&self) -> Result<BlockEngineResponse, BlockEngineError> {
        let response = self
            .client
            .get(BLOCK_EGNINE_RPC)
            .send()
            .await
            .map_err(|e| BlockEngineError::ApiError(format!("{:?}", e)))?;
        if !response.status().is_success() {
            return Err(BlockEngineError::ApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response
                    .text()
                    .await
                    .map_err(|e| BlockEngineError::ApiError(format!("{:?}", e)))?
            )));
        }
        let engine_response: BlockEngineResponse = response
            .json()
            .await
            .map_err(|e| BlockEngineError::ApiError(format!("{:?}", e)))?;
        Ok(engine_response)
    }

    pub async fn get_current_leaders(&self) -> Result<Vec<Leader>, BlockEngineError> {
        let info = self.get_block_engine_info().await?;
        Ok(info.leaders)
    }

    pub async fn get_network_congestion(&self) -> Result<f64, BlockEngineError> {
        let info = self.get_block_engine_info().await?;
        Ok(info.congestion)
    }
}

/// ============== Validators Client ==============
use crate::global::VALIDATORS_RPC;

#[derive(Debug, Clone)]
pub struct ValidatorsClient {
    client: Client,
}

#[derive(Debug)]
pub enum ValidatorsError {
    RequestFailed(String),
    ApiError(String),
}

#[derive(Debug, Deserialize)]
struct ValidatorsResponse {
    validators: Vec<Validator>,
}

#[derive(Debug, Deserialize)]
pub struct Validator {
    pub identity: String,
    pub vote_account: String,
    pub commission: u8,
    pub active: bool,
}

impl ValidatorsClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_validators(&self) -> Result<Vec<Validator>, ValidatorsError> {
        let response = self
            .client
            .get(VALIDATORS_RPC)
            .send()
            .await
            .map_err(|e| ValidatorsError::ApiError(format!("{:?}", e)))?;
        if !response.status().is_success() {
            return Err(ValidatorsError::ApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response
                    .text()
                    .await
                    .map_err(|e| ValidatorsError::ApiError(format!("{:?}", e)))?
            )));
        }
        let validators_response: ValidatorsResponse = response
            .json()
            .await
            .map_err(|e| ValidatorsError::ApiError(format!("{:?}", e)))?;
        Ok(validators_response.validators)
    }

    pub async fn get_active_validators(&self) -> Result<Vec<Validator>, ValidatorsError> {
        let validators = self.get_validators().await?;
        Ok(validators.into_iter().filter(|v| v.active).collect())
    }
}
/// ============== Transactions Pool Client ==============

#[derive(Debug, Clone)]
pub struct TransactionsPoolClient {
    client: Client,
}

#[derive(Debug)]
pub enum TransactionsPoolError {
    RequestFailed(String),
    ApiError(String),
}

#[derive(Debug, Deserialize)]
struct TransactionsResponse {
    transactions: Vec<MemPoolTransaction>,
}

#[derive(Debug, Deserialize)]
pub struct MemPoolTransaction {
    pub signature: String,
    pub slot: u64,
    pub cu_consumed: Option<u64>,
    pub priority_fee: Option<u64>,
}

impl TransactionsPoolClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_mempool_transactions(
        &self,
    ) -> Result<Vec<MemPoolTransaction>, TransactionsPoolError> {
        let response = self
            .client
            .get(TRANSACTIONS_POOL_RPC)
            .send()
            .await
            .map_err(|e| TransactionsPoolError::ApiError(format!("{:?}", e)))?;

        if !response.status().is_success() {
            return Err(TransactionsPoolError::ApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response
                    .text()
                    .await
                    .map_err(|e| TransactionsPoolError::ApiError(format!("{:?}", e)))?
            )));
        }

        let tx_response: TransactionsResponse = response
            .json()
            .await
            .map_err(|e| TransactionsPoolError::ApiError(format!("{:?}", e)))?;
        Ok(tx_response.transactions)
    }

    pub async fn get_high_priority_transactions(
        &self,
        min_priority_fee: u64,
    ) -> Result<Vec<MemPoolTransaction>, TransactionsPoolError> {
        let transactions = self.get_mempool_transactions().await?;
        Ok(transactions
            .into_iter()
            .filter(|tx| tx.priority_fee.unwrap_or(0) >= min_priority_fee)
            .collect())
    }
}

/// ============== Health Client ==============
use crate::global::HEALTH_RPC;

#[derive(Debug, Clone)]
pub struct HealthClient {
    client: Client,
}

#[derive(Debug)]
pub enum HealthError {
    RequestFailed(String),
    ApiError(String),
    Unhealthy(String),
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime: Option<u64>,
}

impl HealthClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn check_health(&self) -> Result<HealthResponse, HealthError> {
        let response = self
            .client
            .get(HEALTH_RPC)
            .send()
            .await
            .map_err(|e| HealthError::ApiError(format!("{:?}", e)))?;

        if !response.status().is_success() {
            return Err(HealthError::ApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response
                    .text()
                    .await
                    .map_err(|e| HealthError::ApiError(format!("{:?}", e)))?
            )));
        }

        let health_response: HealthResponse = response
            .json()
            .await
            .map_err(|e| HealthError::ApiError(format!("{:?}", e)))?;

        if health_response.status != "healthy" {
            return Err(HealthError::Unhealthy(health_response.status));
        }

        Ok(health_response)
    }

    pub async fn is_healthy(&self) -> bool {
        self.check_health().await.is_ok()
    }
}

/// ============== Statistics Client ==============
use crate::global::STATISTICS_RPC;

#[derive(Debug, Clone)]
pub struct StatisticsClient {
    client: Client,
}

#[derive(Debug)]
pub enum StatisticsError {
    RequestFailed(String),
    ApiError(String),
}

#[derive(Debug, Deserialize)]
pub struct StatsResponse {
    pub bundles_sent: u64,
    pub bundles_accepted: u64,
    pub success_rate: f64,
    pub average_tip: u64,
    pub total_volume: u64,
}

impl StatisticsClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    pub async fn get_statistics(&self) -> Result<StatsResponse, StatisticsError> {
        let response = self
            .client
            .get(STATISTICS_RPC)
            .send()
            .await
            .map_err(|e| StatisticsError::ApiError(format!("{:?}", e)))?;
        if !response.status().is_success() {
            return Err(StatisticsError::ApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response
                    .text()
                    .await
                    .map_err(|e| StatisticsError::ApiError(format!("{:?}", e)))?
            )));
        }
        let stats_response: StatsResponse = response
            .json()
            .await
            .map_err(|e| StatisticsError::ApiError(format!("{:?}", e)))?;
        Ok(stats_response)
    }
    pub async fn get_success_rate(&self) -> Result<f64, StatisticsError> {
        let stats = self.get_statistics().await?;
        Ok(stats.success_rate)
    }
}
