use futures_util::StreamExt;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{error, info, warn, debug};

// Import from our mock geyser module
use crate::geyser::{
    GeyserGrpcClient, yellowstone_grpc_proto::prelude::*
};

#[derive(Error, Debug)]
pub enum YellowstoneError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),
    #[error("Stream error: {0}")]
    StreamError(String),
    #[error("Invalid update format: {0}")]
    InvalidUpdate(String),
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
}

#[derive(Debug, Clone)]
pub struct AccountUpdate {
    pub address: String,
    pub lamports: u64,
    pub owner: String,
    pub slot: u64,
    pub data: Vec<u8>,
    pub executable: bool,
}

#[derive(Debug, Clone)]
pub struct TransactionUpdate {
    pub signature: String,
    pub slot: u64,
    pub accounts: Vec<String>,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub logs: Vec<String>,
    pub transaction_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum YellowstoneUpdate {
    Account(AccountUpdate),
    Transaction(TransactionUpdate),
}

// Mock interceptor for tonic
#[derive(Clone)]
#[derive(Debug)]
pub struct MockInterceptor;

impl tonic::service::Interceptor for MockInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        Ok(request)
    }
}

#[derive(Debug)]
pub struct YellowstoneClient {
    endpoint: String,
    token: Option<String>,
    client: Option<GeyserGrpcClient<MockInterceptor>>,
    monitored_addresses: Vec<String>,
    commitment: CommitmentLevel,
}

impl YellowstoneClient {
    pub async fn new(endpoint: &str, token: Option<String>) -> Result<Self, YellowstoneError> {
        info!("Creating Yellowstone client for endpoint: {}", endpoint);
        
        let client = GeyserGrpcClient::<MockInterceptor>::build_from_shared(endpoint.to_string())
            .map_err(|e| YellowstoneError::ConnectionFailed(e.to_string()))?
            .x_token(token.as_deref())
            .map_err(|e| YellowstoneError::AuthFailed(e.to_string()))?
            .connect::<MockInterceptor>()
            .await
            .map_err(|e| YellowstoneError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            endpoint: endpoint.to_string(),
            token,
            client: Some(client),
            monitored_addresses: Vec::new(),
            commitment: CommitmentLevel::Confirmed,
        })
    }

    pub async fn subscribe_to_addresses(&mut self, addresses: Vec<String>) -> Result<(), YellowstoneError> {
        if addresses.is_empty() {
            warn!("No addresses provided for subscription");
            return Ok(());
        }

        info!("Subscribing to {} addresses", addresses.len());
        self.monitored_addresses = addresses.clone();

        let client = self.client.as_mut()
            .ok_or_else(|| YellowstoneError::SubscriptionFailed("Client not initialized".to_string()))?;

        // Create account subscription filters
        let mut account_filters = HashMap::new();
        
        // Monitor all provided addresses for balance changes
        for address in &addresses {
            account_filters.insert(
                format!("account_{}", address),
                SubscribeRequestFilterAccounts {
                    account: vec![address.clone()],
                    owner: vec![], // Monitor any owner
                    filters: vec![SubscribeRequestAccountsDataSlice {
                        offset: 0,
                        length: 0, // We only care about balance changes, not data
                    }],
                },
            );
        }

        // Create transaction subscription filters
        let mut transaction_filters = HashMap::new();
        transaction_filters.insert(
            "transactions".to_string(),
            SubscribeRequestFilterTransactions {
                vote: Some(false), // Exclude vote transactions
                failed: Some(false), // Only successful transactions
                signature: None,
                account_include: addresses.clone(), // Only transactions involving our addresses
                account_exclude: vec![],
                account_required: vec![],
            },
        );

        let _subscription_request = SubscribeRequest {
            accounts: account_filters,
            slots: HashMap::new(),
            transactions: transaction_filters,
            transactions_status: HashMap::new(),
            blocks: HashMap::new(),
            blocks_meta: HashMap::new(),
            entry: HashMap::new(),
            commitment: Some(self.commitment.clone() as i32),
            accounts_data_slice: vec![],
            ping: None,
        };

        // Send subscription request
        let (_sink, mut stream) = client.subscribe().await
            .map_err(|e| YellowstoneError::SubscriptionFailed(e.to_string()))?;

        // Store a reference to know we have an active subscription
        // In a real implementation, we'd store the sink and stream
        info!("Successfully subscribed to {} addresses", addresses.len());

        // For the mock implementation, we'll simulate receiving updates
        // In a real implementation, this would be handled by the actual stream
        tokio::spawn(async move {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(update) => {
                        debug!("Received mock update: {:?}", update);
                        // In the real implementation, these updates would be processed
                        // by the calling code through next_update()
                    }
                    Err(e) => {
                        error!("Stream error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn add_addresses(&mut self, new_addresses: Vec<String>) -> Result<(), YellowstoneError> {
        if new_addresses.is_empty() {
            return Ok(());
        }

        info!("Adding {} new addresses to subscription", new_addresses.len());
        
        let mut all_addresses = self.monitored_addresses.clone();
        all_addresses.extend(new_addresses);
        
        // Remove duplicates
        all_addresses.sort();
        all_addresses.dedup();

        // Resubscribe with updated address list
        self.subscribe_to_addresses(all_addresses).await
    }

    pub async fn next_update(&mut self) -> Result<Option<YellowstoneUpdate>, YellowstoneError> {
        // For the mock implementation, simulate receiving periodic updates
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        if !self.monitored_addresses.is_empty() {
            // Generate a mock account update
            let address = self.monitored_addresses[0].clone();
            let mock_balance = rand::random::<u64>() % 10_000_000_000; // Random balance up to 10 SOL
            
            debug!("Generating mock update for address: {}", address);
            
            Ok(Some(YellowstoneUpdate::Account(AccountUpdate {
                address,
                lamports: mock_balance,
                owner: "11111111111111111111111111111111".to_string(),
                slot: rand::random::<u64>() % 1_000_000,
                data: vec![],
                executable: false,
            })))
        } else {
            debug!("No addresses monitored, returning None");
            Ok(None)
        }
    }

    pub async fn health_check(&mut self) -> Result<bool, YellowstoneError> {
        let client = self.client.as_mut()
            .ok_or_else(|| YellowstoneError::ConnectionFailed("Client not initialized".to_string()))?;

        match client.ping(1).await {
            Ok(_) => {
                debug!("Yellowstone health check successful");
                Ok(true)
            }
            Err(e) => {
                warn!("Yellowstone health check failed: {}", e);
                Ok(false)
            }
        }
    }

    pub fn get_monitored_addresses(&self) -> &[String] {
        &self.monitored_addresses
    }

    pub fn set_commitment(&mut self, commitment: CommitmentLevel) {
        info!("Updated commitment level to: {:?}", commitment);
        self.commitment = commitment;
    }

    pub async fn reconnect(&mut self) -> Result<(), YellowstoneError> {
        info!("Attempting to reconnect to Yellowstone...");
        
        let client = GeyserGrpcClient::<MockInterceptor>::build_from_shared(self.endpoint.clone())
            .map_err(|e| YellowstoneError::ConnectionFailed(e.to_string()))?
            .x_token(self.token.as_deref())
            .map_err(|e| YellowstoneError::AuthFailed(e.to_string()))?
            .connect::<MockInterceptor>()
            .await
            .map_err(|e| YellowstoneError::ConnectionFailed(e.to_string()))?;

        self.client = Some(client);
        
        // Resubscribe to addresses if we had any
        if !self.monitored_addresses.is_empty() {
            let addresses = self.monitored_addresses.clone();
            self.subscribe_to_addresses(addresses).await?;
        }

        info!("Successfully reconnected to Yellowstone");
        Ok(())
    }
}

impl Drop for YellowstoneClient {
    fn drop(&mut self) {
        info!("Dropping Yellowstone client connection");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_yellowstone_client_creation() {
        let client = YellowstoneClient::new("http://localhost:10000", None).await;
        assert!(client.is_ok());
    }

    #[test]
    fn test_account_update_structure() {
        let update = AccountUpdate {
            address: "11111111111111111111111111111111".to_string(),
            lamports: 1000000,
            owner: "11111111111111111111111111111111".to_string(),
            slot: 123456,
            data: vec![],
            executable: false,
        };
        
        assert_eq!(update.lamports, 1000000);
        assert_eq!(update.slot, 123456);
    }
}