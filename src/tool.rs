use std::sync::Arc;

use crate::ArbitrageOpportunity;
use crate::Jito;
use crate::JitoError;
use solana_program::example_mocks::solana_sdk::system_instruction;
use solana_sdk::signer::Signer;
use solana_sdk::{message::Message, pubkey::Pubkey, signature::Keypair, transaction::Transaction};

/// build arbitrage transactions
pub async fn build_arbitrage_transactions(
    jito: Arc<Jito>,
    wallet: &Keypair,
    opportunity: &ArbitrageOpportunity,
    tip_account: Pubkey,
    tip_amount: u64,
) -> Result<Vec<Transaction>, JitoError<String>> {
    let mut transactions = Vec::new();
    let arbitrage_tx = build_dex_swap_transaction(jito, wallet, opportunity).await?;
    transactions.push(arbitrage_tx);
    let tip_tx = build_tip_transaction(wallet, tip_account, tip_amount).await?;
    transactions.push(tip_tx);
    Ok(transactions)
}

/// build dex swap transaction
pub async fn build_dex_swap_transaction(
    jito: Arc<Jito>,
    wallet: &Keypair,
    opportunity: &ArbitrageOpportunity,
) -> Result<Transaction, JitoError<String>> {
    todo!();
    let recent_blockhash = jito
        .solana
        .client_arc()
        .get_latest_blockhash()
        .await
        .map_err(|e| JitoError::Error(format!("{:?}", e)))?;
    let message = Message::new_with_blockhash(
        &[], // 实际的 swap instructions
        Some(&wallet.pubkey()),
        &recent_blockhash,
    );
    Ok(Transaction::new(&[wallet], message, recent_blockhash))
}

pub async fn build_tip_transaction(
    wallet: &Keypair,
    tip_account: Pubkey,
    tip_amount: u64,
) -> Result<Transaction, JitoError<String>> {
    let recent_blockhash = /* 从您的 Solana RPC 获取 */ solana_sdk::hash::Hash::default();
    let tip_instruction = system_instruction::transfer(&wallet.pubkey(), &tip_account, tip_amount);
    let message = Message::new(&[tip_instruction], Some(&wallet.pubkey()));
    Ok(Transaction::new(&[wallet], message, recent_blockhash))
}

// calculate optimal tip
fn cal_optimal_tip(expected_profit: u64, network_congestion: f64, tip_percentage: f64) -> u64 {
    let base_tip = (expected_profit as f64 * tip_percentage).min(1_000_000.0) as u64;
    let congestion_multiplier = 1.0 + network_congestion;
    (base_tip as f64 * congestion_multiplier) as u64
}
