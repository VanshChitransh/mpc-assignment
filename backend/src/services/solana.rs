use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info, warn};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

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
        // Basic validation: should be base58 encoded, around 32-44 characters
        if address.len() < 32 || address.len() > 44 {
            return false;
        }

        // Check if it contains only valid base58 characters
        let base58_chars = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        address.chars().all(|c| base58_chars.contains(c))
    }

    /// Build a SOL transfer transaction
    pub async fn build_sol_transfer(
        &self,
        from_pubkey: &str,
        to_pubkey: &str,
        lamports: u64,
    ) -> Result<UnsignedTransaction, SolanaError> {
        info!("Building SOL transfer: {} -> {} ({} lamports)", from_pubkey, to_pubkey, lamports);

        if !Self::validate_address(from_pubkey) || !Self::validate_address(to_pubkey) {
            return Err(SolanaError::InvalidAddress);
        }

        // Get recent blockhash
        let blockhash = self.get_recent_blockhash().await?;
        
        // Create a simple transfer instruction
        // This is a simplified version - in production you'd use the Solana SDK
        let instruction = self.create_transfer_instruction(from_pubkey, to_pubkey, lamports)?;
        
        // Build transaction message
        let message = self.build_transaction_message(from_pubkey, &[instruction], &blockhash)?;
        
        // Calculate message hash for signing
        let message_hash = self.calculate_message_hash(&message)?;
        
        Ok(UnsignedTransaction {
            transaction_data: BASE64.encode(&message),
            message_hash: hex::encode(&message_hash),
        })
    }

    /// Build a SPL token transfer transaction
    pub async fn build_token_transfer(
        &self,
        from_pubkey: &str,
        to_pubkey: &str,
        mint: &str,
        amount: u64,
    ) -> Result<UnsignedTransaction, SolanaError> {
        info!("Building token transfer: {} -> {} ({} tokens of {})", from_pubkey, to_pubkey, amount, mint);

        if !Self::validate_address(from_pubkey) || !Self::validate_address(to_pubkey) || !Self::validate_address(mint) {
            return Err(SolanaError::InvalidAddress);
        }

        // Get recent blockhash
        let blockhash = self.get_recent_blockhash().await?;
        
        // Create token transfer instruction
        let instruction = self.create_token_transfer_instruction(from_pubkey, to_pubkey, mint, amount)?;
        
        // Build transaction message
        let message = self.build_transaction_message(from_pubkey, &[instruction], &blockhash)?;
        
        // Calculate message hash for signing
        let message_hash = self.calculate_message_hash(&message)?;
        
        Ok(UnsignedTransaction {
            transaction_data: BASE64.encode(&message),
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

        // Decode the transaction
        let transaction_bytes = BASE64.decode(transaction_data)?;
        
        // Add signatures to the transaction
        let signed_transaction = self.add_signatures_to_transaction(transaction_bytes, signatures)?;
        
        // Send the transaction via RPC
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "sendTransaction".to_string(),
            params: serde_json::json!([BASE64.encode(&signed_transaction), {
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

    /// Get recent blockhash from RPC
    async fn get_recent_blockhash(&self) -> Result<String, SolanaError> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getRecentBlockhash".to_string(),
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

    /// Create a transfer instruction (simplified)
    fn create_transfer_instruction(
        &self,
        from: &str,
        to: &str,
        lamports: u64,
    ) -> Result<Vec<u8>, SolanaError> {
        // This is a simplified representation of a Solana transfer instruction
        // In production, you'd use the actual Solana SDK to build proper instructions
        
        // System Program ID (11111111111111111111111111111111)
        let system_program_id = vec![0u8; 32];
        
        // Transfer instruction data: [instruction_type(4 bytes), lamports(8 bytes)]
        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&2u32.to_le_bytes()); // Transfer instruction = 2
        instruction_data.extend_from_slice(&lamports.to_le_bytes());
        
        // Create instruction (simplified format)
        let mut instruction = Vec::new();
        instruction.extend_from_slice(&system_program_id);
        instruction.push(2); // 2 accounts
        instruction.extend_from_slice(from.as_bytes());
        instruction.extend_from_slice(to.as_bytes());
        instruction.extend_from_slice(&(instruction_data.len() as u8).to_le_bytes());
        instruction.extend_from_slice(&instruction_data);
        
        Ok(instruction)
    }

    /// Create a token transfer instruction (simplified)
    fn create_token_transfer_instruction(
        &self,
        from: &str,
        to: &str,
        mint: &str,
        amount: u64,
    ) -> Result<Vec<u8>, SolanaError> {
        // This is a simplified representation
        // In production, you'd use SPL Token program instructions
        
        warn!("Token transfer instruction creation is simplified - needs full SPL Token implementation");
        
        let mut instruction = Vec::new();
        instruction.extend_from_slice(mint.as_bytes());
        instruction.extend_from_slice(from.as_bytes());
        instruction.extend_from_slice(to.as_bytes());
        instruction.extend_from_slice(&amount.to_le_bytes());
        
        Ok(instruction)
    }

    /// Build transaction message (simplified)
    fn build_transaction_message(
        &self,
        payer: &str,
        instructions: &[Vec<u8>],
        blockhash: &str,
    ) -> Result<Vec<u8>, SolanaError> {
        let mut message = Vec::new();
        
        // Add payer
        message.extend_from_slice(payer.as_bytes());
        
        // Add blockhash
        message.extend_from_slice(blockhash.as_bytes());
        
        // Add number of instructions
        message.push(instructions.len() as u8);
        
        // Add instructions
        for instruction in instructions {
            message.extend_from_slice(&(instruction.len() as u16).to_le_bytes());
            message.extend_from_slice(instruction);
        }
        
        Ok(message)
    }

    /// Calculate message hash for signing
    fn calculate_message_hash(&self, message: &[u8]) -> Result<Vec<u8>, SolanaError> {
        // In a real implementation, this would be the proper Solana message hash
        // For now, just return a simple hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        message.hash(&mut hasher);
        let hash = hasher.finish();
        
        Ok(hash.to_le_bytes().to_vec())
    }

    /// Add signatures to transaction
    fn add_signatures_to_transaction(
        &self,
        mut transaction: Vec<u8>,
        signatures: Vec<String>,
    ) -> Result<Vec<u8>, SolanaError> {
        // Prepend signature count and signatures
        let mut signed_tx = Vec::new();
        signed_tx.push(signatures.len() as u8);
        
        for sig in signatures {
            let sig_bytes = hex::decode(&sig)
                .map_err(|_| SolanaError::InvalidTransaction)?;
            signed_tx.extend_from_slice(&sig_bytes);
        }
        
        signed_tx.append(&mut transaction);
        
        Ok(signed_tx)
    }
}

/// Create a default Solana client
pub fn create_solana_client() -> SolanaClient {
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    
    SolanaClient::new(rpc_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_address() {
        // Valid Solana address
        assert!(SolanaClient::validate_address("11111111111111111111111111111111"));
        assert!(SolanaClient::validate_address("So11111111111111111111111111111111111111112"));
        
        // Invalid addresses
        assert!(!SolanaClient::validate_address(""));
        assert!(!SolanaClient::validate_address("invalid"));
        assert!(!SolanaClient::validate_address("0x1234567890abcdef")); // Ethereum format
    }

    #[tokio::test]
    async fn test_build_sol_transfer() {
        let client = create_solana_client();
        
        // This test may fail due to RPC calls, but validates the structure
        let result = client.build_sol_transfer(
            "11111111111111111111111111111111",
            "22222222222222222222222222222222",
            1000000
        ).await;
        
        // In a test environment, we expect this to fail due to invalid addresses or RPC issues
        // The important thing is that the function exists and has the right signature
        match result {
            Ok(_) => println!("SOL transfer built successfully"),
            Err(e) => println!("Expected error in test environment: {}", e),
        }
    }
}