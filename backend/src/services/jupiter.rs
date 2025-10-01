use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, error};
use std::time::Duration;

#[derive(Error, Debug)]
pub enum JupiterError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    
    #[error("Jupiter API error: {0}")]
    JupiterApiError(String),
    
    #[error("No routes available for swap")]
    NoRoutesAvailable,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Quote expired")]
    QuoteExpired,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub inputMint: String,
    pub outputMint: String,
    pub amount: String,
    pub slippageBps: u32,
    pub platformFeeBps: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub inputMint: String,
    pub outputMint: String,
    pub inAmount: String,
    pub outAmount: String,
    pub otherAmountThreshold: String,
    pub swapMode: String,
    pub slippageBps: u32,
    pub platformFee: Option<PlatformFee>,
    pub priceImpactPct: f64,
    pub routePlan: Vec<RoutePlan>,
    pub contextSlot: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    pub feeBps: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutePlan {
    pub percent: u32,
    pub swapInfo: SwapInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapInfo {
    pub ammKey: String,
    pub label: String,
    pub inputMint: String,
    pub outputMint: String,
    pub inAmount: String,
    pub outAmount: String,
    pub feeAmount: String,
    pub feeMint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapRequest {
    pub quoteResponse: QuoteResponse,
    pub userPublicKey: String,
    pub wrapAndUnwrapSol: bool,
    pub autoCompleteMarketOrder: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub swapTransaction: String, // Base64 encoded transaction
}

#[derive(Clone)]
pub struct JupiterClient {
    client: Client,
    base_url: String,
}

impl JupiterClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");
        
        let base_url = std::env::var("JUPITER_API_URL")
            .unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string());
        
        Self {
            client,
            base_url,
        }
    }
    
    /// Get a quote for a token swap
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: &str,
        slippage_percentage: f64,
    ) -> Result<QuoteResponse, JupiterError> {
        if input_mint == output_mint {
            return Err(JupiterError::InvalidInput("Input and output mints cannot be the same".to_string()));
        }
        
        // Convert slippage percentage to basis points (1% = 100 bps)
        let slippage_bps = (slippage_percentage * 100.0) as u32;
        
        let request = QuoteRequest {
            inputMint: input_mint.to_string(),
            outputMint: output_mint.to_string(),
            amount: amount.to_string(),
            slippageBps: slippage_bps,
            platformFeeBps: None, // No platform fee
        };
        
        let url = format!("{}/quote", self.base_url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(JupiterError::JupiterApiError(format!("HTTP {}: {}", status, error_text)));
        }
        
        let quote_response: QuoteResponse = response.json().await?;
        
        Ok(quote_response)
    }
    
    /// Get a swap transaction for a quote
    pub async fn get_swap_transaction(
        &self,
        quote: &QuoteResponse,
        user_public_key: &str,
    ) -> Result<String, JupiterError> {
        let request = SwapRequest {
            quoteResponse: quote.clone(),
            userPublicKey: user_public_key.to_string(),
            wrapAndUnwrapSol: true, // Automatically wrap/unwrap SOL
            autoCompleteMarketOrder: true, // Automatically complete market orders
        };
        
        let url = format!("{}/swap", self.base_url);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(JupiterError::JupiterApiError(format!("HTTP {}: {}", status, error_text)));
        }
        
        let swap_response: SwapResponse = response.json().await?;
        
        Ok(swap_response.swapTransaction)
    }
}

pub fn create_jupiter_client() -> JupiterClient {
    JupiterClient::new()
}