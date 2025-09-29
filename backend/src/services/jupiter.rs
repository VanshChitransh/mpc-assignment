use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum JupiterError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Jupiter API error: {0}")]
    ApiError(String),
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    #[error("No routes found for this swap")]
    NoRoutesFound,
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

// Jupiter API request/response structures
#[derive(Debug, Serialize)]
pub struct JupiterQuoteRequest {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "amount")]
    pub amount: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16, // 50 = 0.5%
    #[serde(rename = "swapMode")]
    pub swap_mode: String, // "ExactIn" or "ExactOut"
}

#[derive(Debug, Deserialize, Serialize)] // Added Serialize trait here
pub struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<Value>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<Value>,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct JupiterSwapRequest {
    #[serde(rename = "quoteResponse")]
    pub quote_response: Value,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
}

#[derive(Debug, Deserialize)]
pub struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String, // Base64 encoded transaction
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: Option<u64>,
}

pub struct JupiterClient {
    client: Client,
    base_url: String,
    default_slippage_bps: u16,
}

impl JupiterClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            default_slippage_bps: 50, // 0.5%
        }
    }

    /// Get a quote for a token swap
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: Option<u16>,
    ) -> Result<JupiterQuoteResponse, JupiterError> {
        let slippage = slippage_bps.unwrap_or(self.default_slippage_bps);
        
        info!(
            "Getting Jupiter quote: {} {} -> {}, slippage: {}bps",
            amount, input_mint, output_mint, slippage
        );

        let request = JupiterQuoteRequest {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            amount: amount.to_string(),
            slippage_bps: slippage,
            swap_mode: "ExactIn".to_string(),
        };

        let url = format!("{}/quote", self.base_url);
        
        let response = self
            .client
            .get(&url)
            .query(&[
                ("inputMint", &request.input_mint),
                ("outputMint", &request.output_mint),
                ("amount", &request.amount),
                ("slippageBps", &request.slippage_bps.to_string()),
                ("swapMode", &request.swap_mode),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Jupiter quote request failed: {}", error_text);
            return Err(JupiterError::ApiError(error_text));
        }

        let quote_response: JupiterQuoteResponse = response.json().await
            .map_err(|e| JupiterError::InvalidResponse(format!("Failed to parse quote response: {}", e)))?;

        info!(
            "Received Jupiter quote: {} {} -> {} {}, price impact: {}%",
            quote_response.in_amount,
            input_mint,
            quote_response.out_amount,
            output_mint,
            quote_response.price_impact_pct
        );

        Ok(quote_response)
    }

    /// Get swap transaction for a quote
    pub async fn get_swap_transaction(
        &self,
        quote_response: Value,
        user_public_key: &str,
    ) -> Result<JupiterSwapResponse, JupiterError> {
        info!("Getting swap transaction for user: {}", user_public_key);

        let request = JupiterSwapRequest {
            quote_response,
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: true,
        };

        let url = format!("{}/swap", self.base_url);
        
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Jupiter swap request failed: {}", error_text);
            return Err(JupiterError::ApiError(error_text));
        }

        let swap_response: JupiterSwapResponse = response.json().await
            .map_err(|e| JupiterError::InvalidResponse(format!("Failed to parse swap response: {}", e)))?;

        info!("Received swap transaction, length: {} bytes", swap_response.swap_transaction.len());

        Ok(swap_response)
    }

    /// Helper function to parse price impact as float
    pub fn parse_price_impact(price_impact_str: &str) -> f64 {
        price_impact_str.parse().unwrap_or(0.0)
    }

    /// Helper function to parse amounts as u64
    pub fn parse_amount(amount_str: &str) -> Result<u64, JupiterError> {
        amount_str.parse()
            .map_err(|_| JupiterError::InvalidResponse(format!("Invalid amount: {}", amount_str)))
    }
}

// Default Jupiter client factory
pub fn create_jupiter_client() -> JupiterClient {
    let base_url = std::env::var("JUPITER_API_URL")
        .unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string());
    
    JupiterClient::new(base_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jupiter_quote() {
        // This test requires internet connection and Jupiter API to be available
        let client = create_jupiter_client();
        
        // Test SOL to USDC quote
        let result = client.get_quote(
            "So11111111111111111111111111111111111111112", // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
            1_000_000_000, // 1 SOL
            Some(50) // 0.5% slippage
        ).await;

        match result {
            Ok(quote) => {
                println!("Quote successful: {} -> {}", quote.in_amount, quote.out_amount);
                assert!(!quote.out_amount.is_empty());
            }
            Err(e) => {
                println!("Quote failed (expected in test environment): {}", e);
                // Don't fail the test since Jupiter API might not be available
            }
        }
    }
}