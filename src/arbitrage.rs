use crate::types::JitoError;
use crate::{ArbitrageConfig, ArbitrageOpportunity, Jito, tool};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::{str::FromStr, sync::Arc};
use tokio::time::{Duration, sleep};

pub struct Arbitrage {
    jito: Arc<Jito>,
}
impl Arbitrage {
    /// create a new Bundler
    pub fn new(jito: Jito) -> Self {
        Self {
            jito: Arc::new(jito),
        }
    }
    pub async fn exe_arbitrage(
        &self,
        wallet: &Keypair,
        opportunity: &ArbitrageOpportunity,
        config: &ArbitrageConfig,
    ) -> Result<String, JitoError<String>> {
        self.jito.health_check().await?;
        let tip_account = self.jito.get_optimal_tip_account().await?;
        let tip_pubkey = Pubkey::from_str(&tip_account.pubkey)
            .map_err(|e| JitoError::SerializationError(e.to_string()))?;
        let tip_amount = (opportunity.expected_profit as f64 * config.tip_percentage) as u64;
        let arbitrage_txs = tool::build_arbitrage_transactions(
            self.jito.clone(),
            wallet,
            opportunity,
            tip_pubkey,
            tip_amount,
        )
        .await?;
        let bundle_id = self
            .jito
            .bundle
            .send_bundle(arbitrage_txs, Some(tip_pubkey), Some(tip_amount))
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))?;
        Ok(bundle_id)
    }

    pub async fn monitor_and_arbitrage(
        &self,
        wallet: Arc<Keypair>,
        config: ArbitrageConfig,
        monitored_pairs: Vec<(Pubkey, Pubkey)>,
    ) {
        log::info!("Starting arbitrage monitoring...");
        loop {
            match self
                .arbitrage_cycle(&wallet, &config, &monitored_pairs)
                .await
            {
                Ok(bundle_id) => {
                    log::info!("Arbitrage executed successfully: {}", bundle_id);
                }
                Err(JitoError::NoArbitrageOpportunity) => {
                    // continue
                }
                Err(e) => {
                    log::error!("Arbitrage cycle failed: {}", e);
                }
            }
            sleep(Duration::from_secs(2)).await;
        }
    }

    async fn arbitrage_cycle(
        &self,
        wallet: &Keypair,
        config: &ArbitrageConfig,
        monitored_pairs: &[(Pubkey, Pubkey)],
    ) -> Result<String, JitoError<String>> {
        // Scanning arbitrage opportunities
        let opportunities = self
            .scan_arbitrage_opportunities(monitored_pairs, 1_000_000)
            .await?;
        if opportunities.is_empty() {
            return Err(JitoError::NoArbitrageOpportunity);
        }
        // Choose the best opportunity
        let best_opportunity = opportunities
            .into_iter()
            .find(|opp| opp.expected_profit >= config.min_profit_lamports)
            .ok_or(JitoError::NoArbitrageOpportunity)?;
        // execution arbitrage
        self.exe_arbitrage(wallet, &best_opportunity, config).await
    }

    async fn scan_arbitrage_opportunities(
        &self,
        token_pairs: &[(Pubkey, Pubkey)],
        amount: u64,
    ) -> Result<Vec<ArbitrageOpportunity>, JitoError<String>> {
        let mut opportunities = Vec::new();
        for (input_mint, output_mint) in token_pairs {
            todo!();
            // To be implemented. This should call the DEX's pricing interface to discover arbitrage opportunities; in practice, it should integrate with DEXs such as Raydium and Orca.
        }
        // sort by profit
        Ok(opportunities)
    }
}
