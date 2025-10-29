use std::sync::Arc;

use crate::Jito;
use crate::types::JitoError;
use solana_network_sdk::tool::token;
use solana_program::example_mocks::solana_sdk::system_instruction;
use solana_sdk::message::Instruction;
use solana_sdk::{
    message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
};

pub struct Bundle {
    jito: Arc<Jito>,
}
impl Bundle {
    /// create a new Bundler
    pub fn new(jito: Jito) -> Self {
        Self {
            jito: Arc::new(jito),
        }
    }

    /// Simple bundled transaction functionality - send any transaction package
    pub async fn send_bundle(
        &self,
        transactions: Vec<Transaction>,
        tip_account: Option<Pubkey>,
        tip_amount: Option<u64>,
    ) -> Result<String, JitoError<String>> {
        self.jito
            .bundle
            .send_bundle(transactions, tip_account, tip_amount)
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))
    }

    /// Create token transfer bundled transactions
    pub async fn create_token_transfer_bundle(
        &self,
        wallet: &Keypair,
        token_mint: Pubkey,
        from_token_account: Pubkey,
        to_token_account: Pubkey,
        ui_amount: f64,
        decimals: u8,
        tip_account: Option<Pubkey>,
        tip_amount: Option<u64>,
    ) -> Result<String, JitoError<String>> {
        let raw_amount = token::safe_ui_to_raw_result(ui_amount, decimals)
            .map_err(|e| JitoError::BundleError(e))?;
        // Get the latest blockhash
        let recent_blockhash = self
            .jito
            .solana
            .client_arc()
            .get_latest_blockhash()
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))?;
        // Create token transfer instruction
        let transfer_instruction = spl_token_interface::instruction::transfer(
            &spl_token::id(),
            &from_token_account,
            &to_token_account,
            &wallet.pubkey(),
            &[],
            raw_amount,
        )
        .map_err(|e| JitoError::Error(format!("{:?}", e)))?;
        let message = Message::new_with_blockhash(
            &[transfer_instruction],
            Some(&wallet.pubkey()),
            &recent_blockhash,
        );
        let token_transfer_tx = Transaction::new(&[wallet], message, recent_blockhash);
        // Send bundled deal
        self.send_bundle(vec![token_transfer_tx], tip_account, tip_amount)
            .await
    }

    /// Create SOL transfer bundled transaction
    pub async fn create_sol_transfer_bundle(
        &self,
        wallet: &Keypair,
        to_pubkey: Pubkey,
        lamports: u64,
        tip_account: Option<Pubkey>,
        tip_amount: Option<u64>,
    ) -> Result<String, JitoError<String>> {
        let recent_blockhash = self
            .jito
            .solana
            .client_arc()
            .get_latest_blockhash()
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))?;
        // Create SOL transfer instruction
        let transfer_instruction =
            system_instruction::transfer(&wallet.pubkey(), &to_pubkey, lamports);
        let message = Message::new_with_blockhash(
            &[transfer_instruction],
            Some(&wallet.pubkey()),
            &recent_blockhash,
        );
        let sol_transfer_tx = Transaction::new(&[wallet], message, recent_blockhash);
        self.send_bundle(vec![sol_transfer_tx], tip_account, tip_amount)
            .await
    }

    /// Create token trading bundles (e.g., trading on Raydium).
    pub async fn create_token_swap_bundle(
        &self,
        wallet: &Keypair,
        swap_instructions: Vec<Instruction>,
        tip_account: Option<Pubkey>,
        tip_amount: Option<u64>,
    ) -> Result<String, JitoError<String>> {
        let recent_blockhash = self
            .jito
            .solana
            .client_arc()
            .get_latest_blockhash()
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))?;
        let message = Message::new_with_blockhash(
            &swap_instructions,
            Some(&wallet.pubkey()),
            &recent_blockhash,
        );
        let swap_tx = Transaction::new(&[wallet], message, recent_blockhash);
        self.send_bundle(vec![swap_tx], tip_account, tip_amount)
            .await
    }

    /// Creating complex multi-transaction bundles
    pub async fn create_multi_transaction_bundle(
        &self,
        wallet: &Keypair,
        transactions: Vec<Transaction>,
        tip_account: Option<Pubkey>,
        tip_amount: Option<u64>,
    ) -> Result<String, JitoError<String>> {
        self.send_bundle(transactions, tip_account, tip_amount)
            .await
    }

    /// Create a bundled transaction of token transfer + tip
    pub async fn create_token_transfer_with_tip_bundle(
        &self,
        wallet: &Keypair,
        token_mint: Pubkey,
        from_token_account: Pubkey,
        to_token_account: Pubkey,
        token_amount: u64,
        tip_account: Pubkey,
        tip_amount: u64,
    ) -> Result<String, JitoError<String>> {
        let recent_blockhash = self
            .jito
            .solana
            .client_arc()
            .get_latest_blockhash()
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))?;
        let mut transactions = Vec::new();
        // Token transfer transactions
        let transfer_instruction = spl_token_interface::instruction::transfer(
            &spl_token::id(),
            &from_token_account,
            &to_token_account,
            &wallet.pubkey(),
            &[],
            token_amount,
        )
        .map_err(|e| JitoError::Error(format!("{:?}", e)))?;
        let transfer_message = Message::new_with_blockhash(
            &[transfer_instruction],
            Some(&wallet.pubkey()),
            &recent_blockhash,
        );
        let transfer_tx = Transaction::new(&[wallet], transfer_message, recent_blockhash);
        transactions.push(transfer_tx);
        // Tip payment transaction
        let tip_instruction =
            system_instruction::transfer(&wallet.pubkey(), &tip_account, tip_amount);
        let tip_message = Message::new_with_blockhash(
            &[tip_instruction],
            Some(&wallet.pubkey()),
            &recent_blockhash,
        );
        let tip_tx = Transaction::new(&[wallet], tip_message, recent_blockhash);
        transactions.push(tip_tx);
        self.send_bundle(transactions, Some(tip_account), Some(tip_amount))
            .await
    }

    /// Bulk token transfer bundling
    pub async fn create_batch_token_transfers_bundle(
        &self,
        wallet: &Keypair,
        transfers: Vec<TokenTransferRequest>,
        tip_account: Option<Pubkey>,
        tip_amount: Option<u64>,
    ) -> Result<String, JitoError<String>> {
        let recent_blockhash = self
            .jito
            .solana
            .client_arc()
            .get_latest_blockhash()
            .await
            .map_err(|e| JitoError::BundleError(e.to_string()))?;
        let mut transactions = Vec::new();
        for transfer in transfers {
            let transfer_instruction = spl_token_interface::instruction::transfer(
                &spl_token::id(),
                &transfer.from_token_account,
                &transfer.to_token_account,
                &wallet.pubkey(),
                &[],
                transfer.amount,
            )
            .map_err(|e| JitoError::Error(format!("{:?}", e)))?;
            let message = Message::new_with_blockhash(
                &[transfer_instruction],
                Some(&wallet.pubkey()),
                &recent_blockhash,
            );
            let tx = Transaction::new(&[wallet], message, recent_blockhash);
            transactions.push(tx);
        }
        self.send_bundle(transactions, tip_account, tip_amount)
            .await
    }
}

#[derive(Debug, Clone)]
pub struct TokenTransferRequest {
    pub from_token_account: Pubkey,
    pub to_token_account: Pubkey,
    pub amount: u64,
}

impl TokenTransferRequest {
    pub fn new(from_token_account: Pubkey, to_token_account: Pubkey, amount: u64) -> Self {
        Self {
            from_token_account,
            to_token_account,
            amount,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BundleConfig {
    pub tip_account: Option<Pubkey>,
    pub tip_amount: Option<u64>,
    pub priority_fee: Option<u64>,
    pub max_retries: u32,
}

impl Default for BundleConfig {
    fn default() -> Self {
        Self {
            tip_account: None,
            tip_amount: None,
            priority_fee: Some(50_000),
            max_retries: 3,
        }
    }
}
