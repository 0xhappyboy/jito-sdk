use crate::client::{
    BlockEngineClient, BundleClient, BundleStatus, HealthClient, HealthResponse, Leader,
    MemPoolTransaction, StatisticsClient, StatsResponse, TipAccount, TipClient,
    TransactionsPoolClient, Validator, ValidatorsClient,
};
pub mod arbitrage;
pub mod bundle;
pub mod client;
pub mod copytrade;
pub mod global;
pub mod tool;
pub mod types;

use crate::types::{JitoError, JitoResult};
use serde::Deserialize;
use solana_network_sdk::Solana;
use solana_network_sdk::tool::token::safe_sol_to_lamports;
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
};
use std::{fmt, str::FromStr, sync::Arc};
use tokio::time::{Duration, sleep};

pub struct Jito {
    bundle: BundleClient,
    tip: TipClient,
    block_engine: BlockEngineClient,
    validators: ValidatorsClient,
    transactions_pool: TransactionsPoolClient,
    health: HealthClient,
    statistics: StatisticsClient,
    // solana client
    solana: Arc<Solana>,
}

pub struct ArbitrageConfig {
    pub min_profit_lamports: u64,
    pub max_slippage_bps: u16, // basis points
    pub max_retries: u32,
    pub tip_percentage: f64, // percentage of profit to use as tip
}

pub struct BackrunConfig {
    pub min_priority_fee: u64,
    pub max_transactions: usize,
    pub profit_threshold: u64,
}

impl Jito {
    pub fn new() -> JitoResult<Self, String> {
        Ok(Self {
            bundle: BundleClient::new(),
            tip: TipClient::new(),
            block_engine: BlockEngineClient::new(),
            validators: ValidatorsClient::new(),
            transactions_pool: TransactionsPoolClient::new(),
            health: HealthClient::new(),
            statistics: StatisticsClient::new(),
            solana: Arc::new(
                Solana::new(solana_network_sdk::types::Mode::MAIN)
                    .map_err(|e| JitoError::Error(format!("{:?}", e)))?,
            ),
        })
    }

    pub async fn health_check(&self) -> Result<HealthResponse, JitoError<String>> {
        self.health
            .check_health()
            .await
            .map_err(|e| JitoError::HealthError(e.to_string()))
    }

    pub async fn get_statistics(&self) -> Result<StatsResponse, JitoError<String>> {
        self.statistics
            .get_statistics()
            .await
            .map_err(|e| JitoError::StatisticsError(e.to_string()))
    }

    pub async fn get_tip_accounts(&self) -> Result<Vec<TipAccount>, JitoError<String>> {
        self.tip
            .get_tip_accounts()
            .await
            .map_err(|e| JitoError::TipError(e.to_string()))
    }

    pub async fn get_optimal_tip_account(&self) -> Result<TipAccount, JitoError<String>> {
        self.tip
            .get_optimal_tip_account()
            .await
            .map_err(|e| JitoError::TipError(e.to_string()))
    }

    pub async fn get_network_congestion(&self) -> Result<f64, JitoError<String>> {
        self.block_engine
            .get_network_congestion()
            .await
            .map_err(|e| JitoError::BlockEngineError(e.to_string()))
    }

    pub async fn get_current_leaders(&self) -> Result<Vec<Leader>, JitoError<String>> {
        self.block_engine
            .get_current_leaders()
            .await
            .map_err(|e| JitoError::BlockEngineError(e.to_string()))
    }

    pub async fn get_active_validators(&self) -> Result<Vec<Validator>, JitoError<String>> {
        self.validators
            .get_active_validators()
            .await
            .map_err(|e| JitoError::ValidatorsError(e.to_string()))
    }

    pub async fn get_mempool_transactions(
        &self,
    ) -> Result<Vec<MemPoolTransaction>, JitoError<String>> {
        self.transactions_pool
            .get_mempool_transactions()
            .await
            .map_err(|e| JitoError::TransactionsPoolError(e.to_string()))
    }

    // ============== Bundle status monitoring ==============

    pub async fn monitor_bundle_status(
        &self,
        bundle_id: &str,
    ) -> Result<BundleStatus, JitoError<String>> {
        self.bundle
            .get_bundle_status(bundle_id)
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))
    }

    pub async fn wait_for_bundle_confirmation(
        &self,
        bundle_id: &str,
        max_retries: u32,
    ) -> Result<bool, JitoError<String>> {
        for _ in 0..max_retries {
            match self.monitor_bundle_status(bundle_id).await {
                Ok(status) => match status.status.as_str() {
                    "confirmed" => return Ok(true),
                    "failed" => return Ok(false),
                    "expired" => return Ok(false),
                    _ => {}
                },
                Err(e) => {
                    log::warn!("Failed to get bundle status: {}", e);
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
        Ok(false)
    }
}

#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub route: Vec<Pubkey>,
    pub expected_profit: u64,
    pub input_amount: u64,
    pub output_amount: u64,
    pub dexes: Vec<String>,
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            min_profit_lamports: safe_sol_to_lamports(0.00001).unwrap_or(10_000), // 0.00001 SOL
            max_slippage_bps: 50,                                                 // 0.5%
            max_retries: 3,
            tip_percentage: 0.1, // 10% of profit
        }
    }
}

impl Default for BackrunConfig {
    fn default() -> Self {
        Self {
            min_priority_fee: safe_sol_to_lamports(0.00005).unwrap_or(50_000), // 0.00005 SOL
            max_transactions: 5,
            profit_threshold: safe_sol_to_lamports(0.000005).unwrap_or(5_000), // 0.000005 SOL
        }
    }
}
