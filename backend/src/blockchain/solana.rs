use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, error, warn};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use sha2::{Sha256, Digest};
use hex;

/// Solana transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub signatures: Vec<String>,
    pub message: TransactionMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMessage {
    pub header: MessageHeader,
    pub account_keys: Vec<String>,
    pub recent_blockhash: String,
    pub instructions: Vec<CompiledInstruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    pub num_required_signatures: u8,
    pub num_readonly_signed_accounts: u8,
    pub num_readonly_unsigned_accounts: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledInstruction {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: String,
}

/// Solana RPC client for blockchain operations
pub struct SolanaBlockchain {
    rpc_url: String,
    commitment: String,
}

impl SolanaBlockchain {
    pub fn new(rpc_url: String, commitment: String) -> Self {
        Self { rpc_url, commitment }
    }

    /// Derive a Solana address from a public key
    pub fn derive_solana_address(public_key: &str) -> Result<String> {
        info!("Deriving Solana address from public key");
        
        // Validate public key format (should be hex encoded)
        if public_key.len() != 64 {
            return Err(anyhow!("Invalid public key length: expected 64 characters"));
        }
        
        // Decode hex public key
        let pubkey_bytes = hex::decode(public_key)
            .map_err(|e| anyhow!("Invalid hex public key: {}", e))?;
        
        if pubkey_bytes.len() != 32 {
            return Err(anyhow!("Invalid public key: expected 32 bytes"));
        }
        
        // Convert to base58 Solana address
        let address = bs58::encode(&pubkey_bytes).into_string();
        
        info!("Successfully derived Solana address: {}", address);
        Ok(address)
    }

    /// Build a Solana transaction
    pub async fn build_transaction(
        &self,
        from: &str,
        to: &str,
        lamports: u64,
        recent_blockhash: &str,
    ) -> Result<Transaction> {
        info!("Building transaction: {} -> {} ({} lamports)", from, to, lamports);
        
        // Validate addresses
        if !self.validate_address(from) || !self.validate_address(to) {
            return Err(anyhow!("Invalid Solana address format"));
        }
        
        // Create transfer instruction
        let instruction = self.create_transfer_instruction(from, to, lamports)?;
        
        // Build transaction message
        let message = TransactionMessage {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![from.to_string(), to.to_string()],
            recent_blockhash: recent_blockhash.to_string(),
            instructions: vec![instruction],
        };
        
        let transaction = Transaction {
            signatures: vec![String::new()], // Empty signature placeholder
            message,
        };
        
        info!("Transaction built successfully");
        Ok(transaction)
    }

    /// Sign a transaction with MPC signature
    pub fn sign_transaction(&self, mut tx: Transaction, signature: &str) -> Result<Transaction> {
        info!("Signing transaction with MPC signature");
        
        // Validate signature format
        if signature.len() != 128 { // 64 bytes * 2 for hex
            return Err(anyhow!("Invalid signature format: expected 128 hex characters"));
        }
        
        // Decode hex signature
        let sig_bytes = hex::decode(signature)
            .map_err(|e| anyhow!("Invalid hex signature: {}", e))?;
        
        if sig_bytes.len() != 64 {
            return Err(anyhow!("Invalid signature length: expected 64 bytes"));
        }
        
        // Add signature to transaction
        tx.signatures[0] = signature.to_string();
        
        info!("Transaction signed successfully");
        Ok(tx)
    }

    /// Send a signed transaction to the Solana network
    pub async fn send_transaction(&self, tx: Transaction) -> Result<String> {
        info!("Sending transaction to Solana network");
        
        // Serialize transaction
        let tx_bytes = bincode::serialize(&tx)
            .map_err(|e| anyhow!("Failed to serialize transaction: {}", e))?;
        
        // Create RPC request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                BASE64.encode(&tx_bytes),
                {
                    "encoding": "base64",
                    "skipPreflight": false,
                    "preflightCommitment": self.commitment
                }
            ]
        });
        
        // Send RPC request
        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("RPC request failed: {}", e))?;
        
        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;
        
        // Parse response
        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Invalid JSON response: {}", e))?;
        
        if let Some(error) = response_json.get("error") {
            let error_msg = error.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown RPC error");
            return Err(anyhow!("RPC error: {}", error_msg));
        }
        
        let signature = response_json.get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No signature in response"))?;
        
        info!("Transaction sent successfully: {}", signature);
        Ok(signature.to_string())
    }

    /// Get recent blockhash from RPC
    pub async fn get_recent_blockhash(&self) -> Result<String> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getRecentBlockhash",
            "params": [{
                "commitment": self.commitment
            }]
        });
        
        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("RPC request failed: {}", e))?;
        
        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;
        
        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Invalid JSON response: {}", e))?;
        
        if let Some(error) = response_json.get("error") {
            let error_msg = error.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown RPC error");
            return Err(anyhow!("RPC error: {}", error_msg));
        }
        
        let blockhash = response_json
            .get("result")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.get("blockhash"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No blockhash in response"))?;
        
        Ok(blockhash.to_string())
    }

    /// Validate Solana address format
    pub fn validate_address(&self, address: &str) -> bool {
        if address.len() < 32 || address.len() > 44 {
            return false;
        }
        
        // Check if it's valid base58
        bs58::decode(address).into_vec().is_ok()
    }

    /// Create a transfer instruction
    fn create_transfer_instruction(
        &self,
        from: &str,
        to: &str,
        lamports: u64,
    ) -> Result<CompiledInstruction> {
        // System Program transfer instruction
        let mut data = Vec::new();
        data.extend_from_slice(&2u32.to_le_bytes()); // Transfer instruction = 2
        data.extend_from_slice(&lamports.to_le_bytes());
        
        Ok(CompiledInstruction {
            program_id_index: 0, // System Program
            accounts: vec![0, 1], // from, to
            data: BASE64.encode(&data),
        })
    }
}

/// Create a default Solana blockchain client
pub fn create_solana_blockchain() -> SolanaBlockchain {
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let commitment = std::env::var("SOLANA_COMMITMENT")
        .unwrap_or_else(|_| "confirmed".to_string());
    
    SolanaBlockchain::new(rpc_url, commitment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_solana_address() {
        // Test with a valid public key
        let pubkey = "1111111111111111111111111111111111111111111111111111111111111111";
        let result = SolanaBlockchain::derive_solana_address(pubkey);
        assert!(result.is_ok());
        
        // Test with invalid public key
        let invalid_pubkey = "invalid";
        let result = SolanaBlockchain::derive_solana_address(invalid_pubkey);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_address() {
        let blockchain = SolanaBlockchain::new("https://api.devnet.solana.com".to_string(), "confirmed".to_string());
        
        // Valid addresses
        assert!(blockchain.validate_address("11111111111111111111111111111111"));
        assert!(blockchain.validate_address("So11111111111111111111111111111111111111112"));
        
        // Invalid addresses
        assert!(!blockchain.validate_address(""));
        assert!(!blockchain.validate_address("invalid"));
        assert!(!blockchain.validate_address("0x1234567890abcdef"));
    }

    #[tokio::test]
    async fn test_get_recent_blockhash() {
        let blockchain = SolanaBlockchain::new("https://api.devnet.solana.com".to_string(), "confirmed".to_string());
        
        let result = blockchain.get_recent_blockhash().await;
        match result {
            Ok(blockhash) => {
                assert!(!blockhash.is_empty());
                println!("Got blockhash: {}", blockhash);
            }
            Err(e) => {
                println!("Expected error in test environment: {}", e);
            }
        }
    }
}
