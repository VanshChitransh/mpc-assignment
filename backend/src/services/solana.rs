use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use sha2::{Sha256, Digest};

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Invalid address format")]
    InvalidAddress,
    #[error("Invalid transaction data")]
    InvalidTransaction,
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
}

#[derive(Debug, Serialize)]
pub struct UnsignedTransaction {
    pub transaction_data: String, // Base64 encoded transaction
    pub message_hash: String,     // Hash that needs to be signed
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionInstruction {
    pub program_id: String,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

pub struct SolanaClient {
    client: Client,
    rpc_url: String,
}

impl SolanaClient {
    pub fn new(rpc_url: String) -> Self {
        Self {
            client: Client::new(),
            rpc_url,
        }
    }

    /// Validate if a string is a valid Solana address format
    pub fn validate_address(address: &str) -> bool {
        // Basic validation: should be base58 encoded, 32-44 characters
        if address.len() < 32 || address.len() > 44 {
            return false;
        }

        // Check if it contains only valid base58 characters
        let base58_chars = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        address.chars().all(|c| base58_chars.contains(c))
    }

    /// Convert hex public key to Solana address
    pub fn pubkey_to_address(hex_pubkey: &str) -> Result<String, SolanaError> {
        // Remove 0x prefix if present
        let clean_hex = hex_pubkey.strip_prefix("0x").unwrap_or(hex_pubkey);
        
        // Decode hex to bytes
        let pubkey_bytes = hex::decode(clean_hex)
            .map_err(|_| SolanaError::InvalidAddress)?;
        
        if pubkey_bytes.len() != 32 {
            return Err(SolanaError::InvalidAddress);
        }
        
        // Encode as base58
        Ok(bs58::encode(pubkey_bytes).into_string())
    }

    /// Build a SOL transfer transaction
    pub async fn build_sol_transfer(
        &self,
        from_pubkey: &str,
        to_pubkey: &str,
        lamports: u64,
    ) -> Result<UnsignedTransaction, SolanaError> {
        info!("Building SOL transfer: {} -> {} ({} lamports)", from_pubkey, to_pubkey, lamports);

        // Convert hex pubkeys to Solana addresses if needed
        let from_address = if from_pubkey.len() == 64 {
            Self::pubkey_to_address(from_pubkey)?
        } else {
            from_pubkey.to_string()
        };
        
        let to_address = if to_pubkey.len() == 64 {
            Self::pubkey_to_address(to_pubkey)?
        } else {
            to_pubkey.to_string()
        };

        if !Self::validate_address(&from_address) || !Self::validate_address(&to_address) {
            return Err(SolanaError::InvalidAddress);
        }

        // Get recent blockhash
        let blockhash = self.get_recent_blockhash().await?;
        
        // Build transfer transaction using RPC
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "simulateTransaction".to_string(), // We'll use getTransaction instead
            params: serde_json::json!([{
                "from": from_address,
                "to": to_address,
                "lamports": lamports,
                "recentBlockhash": blockhash
            }]),
        };
        
        // For now, create a simple transaction structure
        let tx_data = serde_json::json!({
            "message": {
                "accountKeys": [from_address, to_address, "11111111111111111111111111111111"],
                "header": {
                    "numRequiredSignatures": 1,
                    "numReadonlySignedAccounts": 0,
                    "numReadonlyUnsignedAccounts": 1
                },
                "recentBlockhash": blockhash,
                "instructions": [{
                    "programIdIndex": 2,
                    "accounts": [0, 1],
                    "data": BASE64.encode(self.encode_transfer_instruction(lamports))
                }]
            },
            "signatures": [null]
        });
        
        let tx_bytes = serde_json::to_vec(&tx_data)?;
        let message_hash = self.calculate_message_hash(&tx_bytes);
        
        Ok(UnsignedTransaction {
            transaction_data: BASE64.encode(&tx_bytes),
            message_hash: hex::encode(&message_hash),
        })
    }

    /// Build an SPL token transfer transaction
    pub async fn build_token_transfer(
        &self,
        from_pubkey: &str,
        to_pubkey: &str,
        mint: &str,
        amount: u64,
    ) -> Result<UnsignedTransaction, SolanaError> {
        info!("Building token transfer: {} -> {} ({} tokens of {})", from_pubkey, to_pubkey, amount, mint);

        // Convert hex pubkeys to Solana addresses if needed
        let from_address = if from_pubkey.len() == 64 {
            Self::pubkey_to_address(from_pubkey)?
        } else {
            from_pubkey.to_string()
        };
        
        let to_address = if to_pubkey.len() == 64 {
            Self::pubkey_to_address(to_pubkey)?
        } else {
            to_pubkey.to_string()
        };

        if !Self::validate_address(&from_address) || !Self::validate_address(&to_address) || !Self::validate_address(mint) {
            return Err(SolanaError::InvalidAddress);
        }

        // Get token accounts for both addresses
        let from_token_account = self.get_or_create_token_account(&from_address, mint).await?;
        let to_token_account = self.get_or_create_token_account(&to_address, mint).await?;
        
        // Get recent blockhash
        let blockhash = self.get_recent_blockhash().await?;
        
        // Build SPL token transfer transaction
        let token_program_id = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        
        let tx_data = serde_json::json!({
            "message": {
                "accountKeys": [
                    from_address,
                    from_token_account,
                    to_token_account,
                    token_program_id
                ],
                "header": {
                    "numRequiredSignatures": 1,
                    "numReadonlySignedAccounts": 0,
                    "numReadonlyUnsignedAccounts": 1
                },
                "recentBlockhash": blockhash,
                "instructions": [{
                    "programIdIndex": 3,
                    "accounts": [1, 2, 0],
                    "data": BASE64.encode(self.encode_token_transfer_instruction(amount))
                }]
            },
            "signatures": [null]
        });
        
        let tx_bytes = serde_json::to_vec(&tx_data)?;
        let message_hash = self.calculate_message_hash(&tx_bytes);
        
        Ok(UnsignedTransaction {
            transaction_data: BASE64.encode(&tx_bytes),
            message_hash: hex::encode(&message_hash),
        })
    }

    /// Broadcast a signed transaction
    pub async fn broadcast_transaction(
        &self,
        transaction_data: &str,
        signatures: Vec<String>,
    ) -> Result<String, SolanaError> {
        info!("Broadcasting transaction with {} signatures", signatures.len());

        // Decode transaction
        let tx_bytes = BASE64.decode(transaction_data)?;
        
        // Parse transaction JSON and add signatures
        let mut tx: serde_json::Value = serde_json::from_slice(&tx_bytes)?;
        
        // Add signatures to transaction
        if let Some(sigs) = tx.get_mut("signatures") {
            if let Some(sig_array) = sigs.as_array_mut() {
                sig_array.clear();
                for sig in signatures {
                    sig_array.push(serde_json::Value::String(sig));
                }
            }
        }
        
        // Serialize updated transaction
        let signed_tx = BASE64.encode(serde_json::to_vec(&tx)?);
        
        // Send transaction via RPC
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "sendTransaction".to_string(),
            params: serde_json::json!([signed_tx, {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "finalized"
            }]),
        };

        let response = self.client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;

        let rpc_response: RpcResponse<String> = response.json().await?;
        
        if let Some(error) = rpc_response.error {
            return Err(SolanaError::RpcError(format!("{}: {}", error.code, error.message)));
        }

        let signature = rpc_response.result
            .ok_or_else(|| SolanaError::RpcError("No signature in response".to_string()))?;

        info!("Transaction broadcast successful: {}", signature);
        Ok(signature)
    }

    /// Get balance for an address
    pub async fn get_balance(&self, address: &str) -> Result<u64, SolanaError> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getBalance".to_string(),
            params: serde_json::json!([address, {
                "commitment": "finalized"
            }]),
        };

        let response = self.client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct BalanceResult {
            value: u64,
        }

        let rpc_response: RpcResponse<BalanceResult> = response.json().await?;
        
        if let Some(error) = rpc_response.error {
            return Err(SolanaError::RpcError(format!("{}: {}", error.code, error.message)));
        }

        Ok(rpc_response.result
            .ok_or_else(|| SolanaError::RpcError("No balance in response".to_string()))?
            .value)
    }

    /// Get recent blockhash from RPC
    async fn get_recent_blockhash(&self) -> Result<String, SolanaError> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getLatestBlockhash".to_string(),
            params: serde_json::json!([{
                "commitment": "finalized"
            }]),
        };

        let response = self.client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct BlockhashResult {
            value: BlockhashValue,
        }

        #[derive(Deserialize)]
        struct BlockhashValue {
            blockhash: String,
        }

        let rpc_response: RpcResponse<BlockhashResult> = response.json().await?;
        
        if let Some(error) = rpc_response.error {
            return Err(SolanaError::RpcError(format!("{}: {}", error.code, error.message)));
        }

        Ok(rpc_response.result
            .ok_or_else(|| SolanaError::RpcError("No blockhash in response".to_string()))?
            .value
            .blockhash)
    }

    /// Get or create associated token account
    async fn get_or_create_token_account(&self, owner: &str, mint: &str) -> Result<String, SolanaError> {
        // This would normally use the Associated Token Program
        // For simplicity, we're returning a derived address
        let seed = format!("{}-{}", owner, mint);
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let hash = hasher.finalize();
        Ok(bs58::encode(&hash[..32]).into_string())
    }

    /// Calculate message hash for signing
    fn calculate_message_hash(&self, message: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(message);
        hasher.finalize().to_vec()
    }

    /// Encode transfer instruction data
    fn encode_transfer_instruction(&self, lamports: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&2u32.to_le_bytes()); // Transfer instruction = 2
        data.extend_from_slice(&lamports.to_le_bytes());
        data
    }

    /// Encode token transfer instruction data
    fn encode_token_transfer_instruction(&self, amount: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(3); // Transfer instruction for SPL Token
        data.extend_from_slice(&amount.to_le_bytes());
        data
    }
}

/// Create a default Solana client
pub fn create_solana_client() -> SolanaClient {
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    
    SolanaClient::new(rpc_url)
}
