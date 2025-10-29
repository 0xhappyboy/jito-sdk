use crate::BackrunConfig;
use crate::client::MemPoolTransaction;

use crate::Jito;
use crate::types::JitoError;
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
};
use std::{str::FromStr, sync::Arc};
use tokio::time::{Duration, sleep};
pub struct CopyTrade {
    jito: Arc<Jito>,
}
impl CopyTrade {
    /// create a new Bundler
    pub fn new(jito: Jito) -> Self {
        Self {
            jito: Arc::new(jito),
        }
    }
    pub async fn exe_backrun(
        &self,
        wallet: &Keypair,
        target_transaction: Transaction,
        backrun_tx: Transaction,
        config: &BackrunConfig,
    ) -> Result<String, JitoError<String>> {
        let tip_account = self.jito.get_optimal_tip_account().await?;
        let tip_pubkey = Pubkey::from_str(&tip_account.pubkey)
            .map_err(|e| JitoError::SerializationError(e.to_string()))?;
        let tip_amount = config.min_priority_fee;
        let bundle_id = self
            .jito
            .bundle
            .send_bundle(
                vec![target_transaction, backrun_tx],
                Some(tip_pubkey),
                Some(tip_amount),
            )
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))?;
        log::info!("Backrun bundle sent: {}", bundle_id);
        Ok(bundle_id)
    }

    pub async fn monitor_for_backrun_opportunities(
        &self,
        wallet: Arc<Keypair>,
        config: BackrunConfig,
    ) {
        log::info!("Starting backrun monitoring...");
        loop {
            if let Ok(high_priority_txs) = self
                .jito
                .transactions_pool
                .get_high_priority_transactions(config.min_priority_fee)
                .await
            {
                for target_tx in high_priority_txs.iter().take(config.max_transactions) {
                    if let Ok(backrun_tx) = self
                        .build_backrun_transaction(&wallet, target_tx, config.profit_threshold)
                        .await
                    {
                        if let Err(e) = self
                            .exe_backrun(&wallet, Transaction::default(), backrun_tx, &config)
                            .await
                        {
                            log::error!("Backrun execution failed: {}", e);
                        }
                    }
                }
            }
            sleep(Duration::from_millis(500)).await;
        }
    }

    async fn build_backrun_transaction(
        &self,
        wallet: &Keypair,
        target_tx: &MemPoolTransaction,
        profit_threshold: u64,
    ) -> Result<Transaction, JitoError<String>> {
        // Build copy trading logic, analyze target trades, and construct corresponding copy trading strategies.
        todo!();
        let recent_blockhash = self
            .jito
            .solana
            .client_arc()
            .get_latest_blockhash()
            .await
            .map_err(|e| JitoError::Error(format!("{:?}", e)))?;
        let message = Message::new_with_blockhash(&[], Some(&wallet.pubkey()), &recent_blockhash);
        Ok(Transaction::new(&[wallet], message, recent_blockhash))
    }
}
