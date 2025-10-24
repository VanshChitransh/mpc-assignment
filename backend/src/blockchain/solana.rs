// backend/src/blockchain/solana.rs
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
    message::Message,
    hash::Hash,
    system_instruction,
    native_token::LAMPORTS_PER_SOL,
};
use spl_token::instruction as token_instruction;
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::create_associated_token_account,
};
use tracing::{error, info, warn};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Insufficient balance: required {0}, available {1}")]
    InsufficientBalance(u64, u64),
    
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Encoding error: {0}")]
    EncodingError(String),
}

#[derive(Debug, Clone)]
pub struct SolanaBalance {
    pub lamports: u64,
    pub ui_amount: f64,
}

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub mint: String,
    pub amount: u64,
    pub decimals: u8,
    pub ui_amount: f64,
}

#[derive(Debug, Clone)]
pub struct SolanaClient {
    rpc_client: Arc<RpcClient>,
    mock_mode: bool,
}

impl SolanaClient {
    pub fn new(rpc_url: String, mock_mode: bool) -> Self {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url,
            CommitmentConfig::confirmed(),
        );
        
        Self {
            rpc_client: Arc::new(rpc_client),
            mock_mode,
        }
    }
    
    /// Validate a Solana address format
    pub fn validate_address(&self, address: &str) -> bool {
        Pubkey::from_str(address).is_ok()
    }
    
    /// Derive Solana address from Ed25519 public key (hex format)
    pub fn derive_solana_address(&self, public_key_hex: &str) -> Result<String, SolanaError> {
        if public_key_hex.len() != 64 {
            return Err(SolanaError::InvalidPublicKey(format!(
                "Public key must be 64 hex characters (32 bytes), got {} characters", 
                public_key_hex.len()
            )));
        }
        
        let bytes = hex::decode(public_key_hex)
            .map_err(|e| SolanaError::EncodingError(format!("Failed to decode public key: {}", e)))?;
        
        if bytes.len() != 32 {
            return Err(SolanaError::InvalidPublicKey(
                "Public key must be exactly 32 bytes".to_string()
            ));
        }
        
        let pubkey = Pubkey::new_from_array(
            bytes.try_into().map_err(|_| SolanaError::InvalidPublicKey("Invalid byte array".to_string()))?
        );
        
        Ok(pubkey.to_string())
    }
    
    /// Get SOL balance for an address
    pub async fn get_sol_balance(&self, address: &str) -> Result<SolanaBalance, SolanaError> {
        if self.mock_mode {
            return Ok(SolanaBalance {
                lamports: 1_000_000_000, // 1 SOL
                ui_amount: 1.0,
            });
        }
        
        let pubkey = Pubkey::from_str(address)
            .map_err(|e| SolanaError::InvalidAddress(e.to_string()))?;
        
        let lamports = self.rpc_client
            .get_balance(&pubkey)
            .map_err(|e| SolanaError::RpcError(e.to_string()))?;
        
        Ok(SolanaBalance {
            lamports,
            ui_amount: lamports as f64 / LAMPORTS_PER_SOL as f64,
        })
    }
    
    /// Get token balance for a specific mint
    pub async fn get_token_balance(&self, owner: &str, mint: &str) -> Result<TokenBalance, SolanaError> {
        if self.mock_mode {
            return Ok(TokenBalance {
                mint: mint.to_string(),
                amount: 1000_000_000, // 1000 tokens with 6 decimals
                decimals: 6,
                ui_amount: 1000.0,
            });
        }
        
        let owner_pubkey = Pubkey::from_str(owner)
            .map_err(|e| SolanaError::InvalidAddress(format!("Invalid owner: {}", e)))?;
        
        let mint_pubkey = Pubkey::from_str(mint)
            .map_err(|e| SolanaError::InvalidAddress(format!("Invalid mint: {}", e)))?;
        
        let token_account = get_associated_token_address(&owner_pubkey, &mint_pubkey);
        
        match self.rpc_client.get_token_account_balance(&token_account) {
            Ok(balance) => {
                Ok(TokenBalance {
                    mint: mint.to_string(),
                    amount: balance.amount.parse().unwrap_or(0),
                    decimals: balance.decimals,
                    ui_amount: balance.ui_amount.unwrap_or(0.0),
                })
            },
            Err(_) => {
                // Token account doesn't exist, return zero balance
                Ok(TokenBalance {
                    mint: mint.to_string(),
                    amount: 0,
                    decimals: 6, // Default decimals
                    ui_amount: 0.0,
                })
            }
        }
    }
    
    /// Get recent blockhash
    pub async fn get_recent_blockhash(&self) -> Result<Hash, SolanaError> {
        if self.mock_mode {
            return Ok(Hash::default());
        }
        
        self.rpc_client
            .get_latest_blockhash()
            .map_err(|e| SolanaError::RpcError(e.to_string()))
    }
    
    /// Create a SOL transfer transaction
    pub async fn create_transfer_transaction(
        &self,
        from_address: &str,
        to_address: &str,
        amount: f64,
        memo: Option<String>,
    ) -> Result<Transaction, SolanaError> {
        let from_pubkey = Pubkey::from_str(from_address)
            .map_err(|e| SolanaError::InvalidAddress(format!("Invalid sender: {}", e)))?;
        
        let to_pubkey = Pubkey::from_str(to_address)
            .map_err(|e| SolanaError::InvalidAddress(format!("Invalid recipient: {}", e)))?;
        
        let lamports = (amount * LAMPORTS_PER_SOL as f64) as u64;
        
        // Check balance
        if !self.mock_mode {
            let balance = self.get_sol_balance(from_address).await?;
            let fee_estimate = 5000; // ~5000 lamports for transaction fee
            
            if balance.lamports < lamports + fee_estimate {
                return Err(SolanaError::InsufficientBalance(
                    lamports + fee_estimate,
                    balance.lamports
                ));
            }
        }
        
        let recent_blockhash = self.get_recent_blockhash().await?;
        
        let mut instructions = vec![
            system_instruction::transfer(&from_pubkey, &to_pubkey, lamports)
        ];
        
        // Add memo if provided
        if let Some(memo_text) = memo {
            let memo_program = Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
                .map_err(|e| SolanaError::InvalidAddress(e.to_string()))?;
            
            instructions.push(
                solana_sdk::instruction::Instruction {
                    program_id: memo_program,
                    accounts: vec![],
                    data: memo_text.as_bytes().to_vec(),
                }
            );
        }
        
        let message = Message::new(&instructions, Some(&from_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = recent_blockhash;
        
        Ok(transaction)
    }
    
    /// Create an SPL token transfer transaction
    pub async fn create_token_transfer_transaction(
        &self,
        from_address: &str,
        to_address: &str,
        mint: &str,
        amount: f64,
        decimals: u8,
        memo: Option<String>,
    ) -> Result<Transaction, SolanaError> {
        let from_pubkey = Pubkey::from_str(from_address)
            .map_err(|e| SolanaError::InvalidAddress(format!("Invalid sender: {}", e)))?;
        
        let to_pubkey = Pubkey::from_str(to_address)
            .map_err(|e| SolanaError::InvalidAddress(format!("Invalid recipient: {}", e)))?;
        
        let mint_pubkey = Pubkey::from_str(mint)
            .map_err(|e| SolanaError::InvalidAddress(format!("Invalid mint: {}", e)))?;
        
        let token_amount = (amount * 10_f64.powi(decimals as i32)) as u64;
        
        // Get associated token accounts
        let from_token_account = get_associated_token_address(&from_pubkey, &mint_pubkey);
        let to_token_account = get_associated_token_address(&to_pubkey, &mint_pubkey);
        
        let recent_blockhash = self.get_recent_blockhash().await?;
        
        let mut instructions = vec![];
        
        // Check if destination token account exists, create if not
        if !self.mock_mode {
            if let Err(_) = self.rpc_client.get_account(&to_token_account) {
                instructions.push(
                    create_associated_token_account(
                        &from_pubkey,
                        &to_pubkey,
                        &mint_pubkey,
                        &spl_token::id(),
                    )
                );
            }
        }
        
        // Add transfer instruction
        instructions.push(
            token_instruction::transfer(
                &spl_token::id(),
                &from_token_account,
                &to_token_account,
                &from_pubkey,
                &[],
                token_amount,
            ).map_err(|e| SolanaError::InvalidTransaction(e.to_string()))?
        );
        
        // Add memo if provided
        if let Some(memo_text) = memo {
            let memo_program = Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
                .map_err(|e| SolanaError::InvalidAddress(e.to_string()))?;
            
            instructions.push(
                solana_sdk::instruction::Instruction {
                    program_id: memo_program,
                    accounts: vec![],
                    data: memo_text.as_bytes().to_vec(),
                }
            );
        }
        
        let message = Message::new(&instructions, Some(&from_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = recent_blockhash;
        
        Ok(transaction)
    }
}

// CRITICAL: Export helper functions for transaction signing

/// Extract transaction hash for signing
pub fn extract_transaction_hash(transaction: &Transaction) -> String {
    let message_bytes = transaction.message.serialize();
    // Return first 32 bytes as hex for signing
    hex::encode(&message_bytes[..std::cmp::min(32, message_bytes.len())])
}

/// Sign and finalize transaction with hex signature
pub fn sign_and_finalize_transaction(
    mut transaction: Transaction,
    signature_hex: String,
) -> Result<Transaction, SolanaError> {
    let signature_bytes = hex::decode(&signature_hex)
        .map_err(|e| SolanaError::SerializationError(format!("Invalid signature hex: {}", e)))?;
    
    if signature_bytes.len() != 64 {
        return Err(SolanaError::SerializationError(
            format!("Signature must be 64 bytes, got {}", signature_bytes.len())
        ));
    }
    
    let signature = Signature::try_from(signature_bytes.as_slice())
        .map_err(|e| SolanaError::SerializationError(format!("Invalid signature: {}", e)))?;
    
    transaction.signatures = vec![signature];
    Ok(transaction)
}

/// Broadcast a signed transaction
pub async fn broadcast_transaction(
    rpc_url: &str,
    transaction: Result<Transaction, SolanaError>,
) -> Result<String, SolanaError> {
    let transaction = transaction?;
    
    // Verify transaction has signature
    if transaction.signatures.is_empty() {
        return Err(SolanaError::TransactionFailed("Transaction not signed".to_string()));
    }
    
    // In mock mode or if rpc_url is empty, return mock signature
    if rpc_url.is_empty() {
        let signature = transaction.signatures.first()
            .ok_or_else(|| SolanaError::TransactionFailed("No signature".to_string()))?;
        return Ok(signature.to_string());
    }
    
    // Create RPC client and send transaction
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    );
    
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .map_err(|e| SolanaError::TransactionFailed(format!("Broadcast failed: {}", e)))?;
    
    Ok(signature.to_string())
}

/// Factory function to create SolanaClient
pub fn create_solana_client(rpc_url: String, mock_mode: bool) -> SolanaClient {
    SolanaClient::new(rpc_url, mock_mode)
}