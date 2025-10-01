use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn, error};
use sha2::{Sha256, Digest};
use std::time::Duration;

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),
    
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Encoding error: {0}")]
    EncodingError(String),
    
    #[error("Transaction broadcast failed: {0}")]
    BroadcastFailed(String),
    
    #[error("Insufficient balance: required {0}, available {1}")]
    InsufficientBalance(u64, u64),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub message: Vec<u8>,
    pub signatures: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    pub transaction_data: String,
    pub message_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SolanaBalance {
    pub lamports: u64,
    pub ui_balance: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: String,
    pub amount: String, // String to handle large uint64 values
    pub decimals: u8,
    pub ui_amount: f64,
    pub ui_amount_string: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockhashResponse {
    jsonrpc: String,
    id: u64,
    result: Option<BlockhashResult>,
    error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockhashResult {
    value: BlockhashValue,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockhashValue {
    blockhash: String,
    last_valid_block_height: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransactionResponse {
    jsonrpc: String,
    id: u64,
    result: Option<String>,
    error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BalanceResponse {
    jsonrpc: String,
    id: u64,
    result: Option<BalanceResult>,
    error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BalanceResult {
    context: RpcContext,
    value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcContext {
    slot: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenAccountsResponse {
    jsonrpc: String,
    id: u64,
    result: Option<TokenAccountsResult>,
    error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenAccountsResult {
    context: RpcContext,
    value: Vec<TokenAccountInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenAccountInfo {
    pubkey: String,
    account: TokenAccountData,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenAccountData {
    data: TokenData,
    executable: bool,
    lamports: u64,
    owner: String,
    rent_epoch: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenData {
    parsed: TokenParsed,
    program: String,
    space: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenParsed {
    info: TokenInfo,
    #[serde(rename = "type")]
    token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenInfo {
    mint: String,
    owner: String,
    #[serde(rename = "tokenAmount")]
    token_amount: TokenAmount,
    state: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenAmount {
    amount: String,
    decimals: u8,
    #[serde(rename = "uiAmount")]
    ui_amount: f64,
    #[serde(rename = "uiAmountString")]
    ui_amount_string: String,
}

#[derive(Clone)]
pub struct SolanaBlockchain {
    client: Client,
    rpc_url: String,
    commitment: String,
    token_program_id: String,
    associated_token_program_id: String,
}

impl SolanaBlockchain {
    pub fn new(rpc_url: String, commitment: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            rpc_url,
            commitment,
            token_program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            associated_token_program_id: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".to_string(),
        }
    }
    
    /// Validate a Solana address format
    pub fn validate_address(&self, address: &str) -> bool {
        // Base58 encoded Solana addresses are typically 32-44 characters
        if address.len() < 32 || address.len() > 44 {
            return false;
        }
        
        // Try to decode from base58
        match bs58::decode(address).into_vec() {
            Ok(decoded) => decoded.len() == 32,
            Err(_) => false,
        }
    }
    
    /// Derive Solana address from Ed25519 public key
    pub fn derive_solana_address(&self, public_key: &str) -> Result<String, SolanaError> {
        // Validate the public key format (should be 64 hex characters = 32 bytes)
        if public_key.len() != 64 {
            return Err(SolanaError::InvalidPublicKey(format!(
                "Public key must be 64 hex characters (32 bytes), got {} characters", 
                public_key.len()
            )));
        }
        
        // Convert from hex to bytes
        let bytes = hex::decode(public_key)
            .map_err(|e| SolanaError::EncodingError(format!("Failed to decode public key: {}", e)))?;
        
        // Convert to base58
        let address = bs58::encode(&bytes).into_string();
        
        Ok(address)
    }
    
    /// Get SOL balance for an address
    pub async fn get_sol_balance(&self, address: &str) -> Result<SolanaBalance, SolanaError> {
        if !self.validate_address(address) {
            return Err(SolanaError::InvalidAddress(format!("Invalid address format: {}", address)));
        }
        
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [
                address,
                {"commitment": self.commitment}
            ]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request_body)
            .send()
            .await?;
        
        let balance_response: BalanceResponse = response.json().await?;
        
        if let Some(error) = balance_response.error {
            return Err(SolanaError::RpcError(format!("RPC error: {} - {}", error.code, error.message)));
        }
        
        match balance_response.result {
            Some(result) => {
                let lamports = result.value;
                let ui_balance = lamports as f64 / 1_000_000_000.0; // 9 decimals for SOL
                
                Ok(SolanaBalance {
                    lamports,
                    ui_balance,
                })
            },
            None => Err(SolanaError::RpcError("No result in balance response".to_string())),
        }
    }
    
    /// Get all token balances for an address
    pub async fn get_token_balances(&self, address: &str) -> Result<Vec<TokenBalance>, SolanaError> {
        if !self.validate_address(address) {
            return Err(SolanaError::InvalidAddress(format!("Invalid address format: {}", address)));
        }
        
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                address,
                {
                    "programId": self.token_program_id
                },
                {
                    "encoding": "jsonParsed"
                }
            ]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request_body)
            .send()
            .await?;
        
        let accounts_response: TokenAccountsResponse = response.json().await?;
        
        if let Some(error) = accounts_response.error {
            return Err(SolanaError::RpcError(format!("RPC error: {} - {}", error.code, error.message)));
        }
        
        match accounts_response.result {
            Some(result) => {
                let mut token_balances = Vec::new();
                
                for account in result.value {
                    let token_data = &account.account.data.parsed.info;
                    
                    token_balances.push(TokenBalance {
                        mint: token_data.mint.clone(),
                        amount: token_data.token_amount.amount.clone(),
                        decimals: token_data.token_amount.decimals,
                        ui_amount: token_data.token_amount.ui_amount,
                        ui_amount_string: token_data.token_amount.ui_amount_string.clone(),
                    });
                }
                
                Ok(token_balances)
            },
            None => Err(SolanaError::RpcError("No result in token accounts response".to_string())),
        }
    }
    
    /// Get recent blockhash from Solana network
    pub async fn get_recent_blockhash(&self) -> Result<String, SolanaError> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [{
                "commitment": self.commitment
            }]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request_body)
            .send()
            .await?;
        
        let rpc_response: BlockhashResponse = response.json().await?;
        
        if let Some(error) = rpc_response.error {
            return Err(SolanaError::RpcError(format!("Failed to get blockhash: {} - {}", error.code, error.message)));
        }
        
        match &rpc_response.result {
            Some(result) => Ok(result.value.blockhash.clone()),
            None => Err(SolanaError::RpcError("No blockhash in response".to_string())),
        }
    }
    
    /// Build a SOL transfer transaction
    pub async fn build_sol_transfer(
        &self, 
        from_pubkey: &str, 
        to_pubkey: &str, 
        lamports: u64
    ) -> Result<UnsignedTransaction, SolanaError> {
        // Validate addresses
        if !self.validate_address(from_pubkey) {
            return Err(SolanaError::InvalidAddress(format!("Invalid sender address: {}", from_pubkey)));
        }
        
        if !self.validate_address(to_pubkey) {
            return Err(SolanaError::InvalidAddress(format!("Invalid recipient address: {}", to_pubkey)));
        }
        
        // Check balance
        let balance = self.get_sol_balance(from_pubkey).await?;
        if balance.lamports < lamports + 5000 { // Add 5000 lamports for fee
            return Err(SolanaError::InsufficientBalance(lamports + 5000, balance.lamports));
        }
        
        // Get recent blockhash
        let blockhash = self.get_recent_blockhash().await?;
        
        // System program address
        let system_program = "11111111111111111111111111111111";
        
        // Create transfer instruction
        let transfer_data = self.encode_transfer_instruction(lamports);
        
        // Build transaction message
        let message = self.build_transaction_message(
            from_pubkey, 
            system_program,
            to_pubkey, 
            &transfer_data, 
            &blockhash
        )?;
        
        // Calculate message hash for signing
        let message_hash = hex::encode(self.calculate_message_hash(&message));
        
        // Return unsigned transaction
        Ok(UnsignedTransaction {
            transaction_data: hex::encode(&message),
            message_hash,
        })
    }
    
    /// Build an SPL token transfer transaction
    pub async fn build_token_transfer(
        &self, 
        from_pubkey: &str, 
        to_pubkey: &str, 
        mint: &str, 
        amount: u64
    ) -> Result<UnsignedTransaction, SolanaError> {
        // Validate addresses
        if !self.validate_address(from_pubkey) {
            return Err(SolanaError::InvalidAddress(format!("Invalid sender address: {}", from_pubkey)));
        }
        
        if !self.validate_address(to_pubkey) {
            return Err(SolanaError::InvalidAddress(format!("Invalid recipient address: {}", to_pubkey)));
        }
        
        if !self.validate_address(mint) {
            return Err(SolanaError::InvalidAddress(format!("Invalid mint address: {}", mint)));
        }
        
        // Get recent blockhash
        let blockhash = self.get_recent_blockhash().await?;
        
        // Get token balance to check if sufficient
        let token_balances = self.get_token_balances(from_pubkey).await?;
        let balance = token_balances.iter().find(|b| b.mint == mint);
        
        if let Some(balance) = balance {
            let balance_amount = balance.amount.parse::<u64>().unwrap_or(0);
            if balance_amount < amount {
                return Err(SolanaError::InsufficientBalance(amount, balance_amount));
            }
        } else {
            return Err(SolanaError::InsufficientBalance(amount, 0));
        }
        
        // Get or create source token account
        let source_token_account = self.get_or_create_token_account(from_pubkey, mint).await?;
        
        // Get or create destination token account
        let dest_token_account = self.get_or_create_token_account(to_pubkey, mint).await?;
        
        // Create token transfer instruction
        let transfer_data = self.encode_token_transfer_instruction(amount);
        
        // Build transaction message for token transfer
        let message = self.build_token_transaction_message(
            from_pubkey,
            &source_token_account,
            &dest_token_account,
            &self.token_program_id,
            &transfer_data,
            &blockhash
        )?;
        
        // Calculate message hash for signing
        let message_hash = hex::encode(self.calculate_message_hash(&message));
        
        // Return unsigned transaction
        Ok(UnsignedTransaction {
            transaction_data: hex::encode(&message),
            message_hash,
        })
    }
    
    /// Apply signature and broadcast transaction
    pub async fn broadcast_transaction(
        &self,
        transaction_data: &str,
        signature: &str
    ) -> Result<String, SolanaError> {
        // Decode transaction data
        let message = hex::decode(transaction_data)
            .map_err(|e| SolanaError::EncodingError(format!("Failed to decode transaction data: {}", e)))?;
        
        // Decode signature
        let signature_bytes = hex::decode(signature)
            .map_err(|e| SolanaError::EncodingError(format!("Failed to decode signature: {}", e)))?;
        
        // Build signed transaction
        let signed_tx = self.build_signed_transaction(message, signature_bytes)?;
        
        // Encode as base64 for RPC
        let tx_base64 = base64::encode(&signed_tx);
        
        // Send transaction
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                tx_base64,
                {
                    "encoding": "base64",
                    "commitment": self.commitment,
                    "skipPreflight": false
                }
            ]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .json(&request_body)
            .send()
            .await?;
        
        let rpc_response: TransactionResponse = response.json().await?;
        
        if let Some(error) = rpc_response.error {
            return Err(SolanaError::BroadcastFailed(format!(
                "Failed to broadcast transaction: {} - {}", 
                error.code, 
                error.message
            )));
        }
        
        match rpc_response.result {
            Some(signature) => Ok(signature),
            None => Err(SolanaError::BroadcastFailed("No transaction ID in response".to_string())),
        }
    }
    
    /// Build a signed transaction from message and signature
    fn build_signed_transaction(
        &self,
        message: Vec<u8>,
        signature: Vec<u8>
    ) -> Result<Vec<u8>, SolanaError> {
        // In a real implementation, this would use Solana SDK to construct the transaction
        // For now, we'll build a simplified format
        
        // Signature count (1)
        let mut tx_data = vec![1];
        
        // Signature bytes
        tx_data.extend_from_slice(&signature);
        
        // Message
        tx_data.extend_from_slice(&message);
        
        Ok(tx_data)
    }
    
    /// Build a transaction message
    fn build_transaction_message(
        &self,
        from: &str,
        program_id: &str,
        to: &str,
        instruction_data: &[u8],
        recent_blockhash: &str
    ) -> Result<Vec<u8>, SolanaError> {
        // Decode addresses from base58
        let from_bytes = bs58::decode(from)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress(from.to_string()))?;
        
        let to_bytes = bs58::decode(to)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress(to.to_string()))?;
        
        let program_bytes = bs58::decode(program_id)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress(program_id.to_string()))?;
        
        let blockhash_bytes = bs58::decode(recent_blockhash)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress("Invalid blockhash".to_string()))?;
        
        // Build simplified message format
        let mut message = Vec::new();
        
        // Header (1 required signature, 0 read-only signed, 0 read-only unsigned)
        message.push(1); // Num required signatures
        message.push(0); // Num read-only signed
        message.push(0); // Num read-only unsigned
        
        // Account addresses (3: from, to, program_id)
        message.push(3); // Num account addresses
        message.extend_from_slice(&from_bytes);
        message.extend_from_slice(&to_bytes);
        message.extend_from_slice(&program_bytes);
        
        // Recent blockhash
        message.extend_from_slice(&blockhash_bytes);
        
        // Instructions (1)
        message.push(1); // Num instructions
        
        // Instruction 0
        message.push(2); // Program ID index
        message.push(2); // Accounts array length
        message.push(0); // From account index
        message.push(1); // To account index
        
        // Instruction data length and bytes
        message.push(instruction_data.len() as u8);
        message.extend_from_slice(instruction_data);
        
        Ok(message)
    }
    
    /// Build a token transaction message
    fn build_token_transaction_message(
        &self,
        owner: &str,
        from_token: &str,
        to_token: &str,
        token_program: &str,
        instruction_data: &[u8],
        recent_blockhash: &str
    ) -> Result<Vec<u8>, SolanaError> {
        // Decode addresses from base58
        let owner_bytes = bs58::decode(owner)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress(owner.to_string()))?;
        
        let from_token_bytes = bs58::decode(from_token)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress(from_token.to_string()))?;
        
        let to_token_bytes = bs58::decode(to_token)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress(to_token.to_string()))?;
        
        let program_bytes = bs58::decode(token_program)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress(token_program.to_string()))?;
        
        let blockhash_bytes = bs58::decode(recent_blockhash)
            .into_vec()
            .map_err(|_| SolanaError::InvalidAddress("Invalid blockhash".to_string()))?;
        
        // Build simplified message format for token transfer
        let mut message = Vec::new();
        
        // Header (1 required signature, 0 read-only signed, 2 read-only unsigned)
        message.push(1); // Num required signatures
        message.push(0); // Num read-only signed
        message.push(2); // Num read-only unsigned
        
        // Account addresses (4: owner, from_token, to_token, token_program)
        message.push(4); // Num account addresses
        message.extend_from_slice(&owner_bytes);
        message.extend_from_slice(&from_token_bytes);
        message.extend_from_slice(&to_token_bytes);
        message.extend_from_slice(&program_bytes);
        
        // Recent blockhash
        message.extend_from_slice(&blockhash_bytes);
        
        // Instructions (1)
        message.push(1); // Num instructions
        
        // Instruction 0
        message.push(3); // Program ID index
        message.push(3); // Accounts array length
        message.push(1); // From token account index
        message.push(2); // To token account index
        message.push(0); // Owner account index
        
        // Instruction data length and bytes
        message.push(instruction_data.len() as u8);
        message.extend_from_slice(instruction_data);
        
        Ok(message)
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

/// Create a default Solana blockchain instance
pub fn create_solana_blockchain() -> SolanaBlockchain {
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    
    let commitment = std::env::var("SOLANA_COMMITMENT")
        .unwrap_or_else(|_| "confirmed".to_string());
    
    SolanaBlockchain::new(rpc_url, commitment)
}