#[cfg(test)]
mod tests {
    use backend::blockchain::solana::{SolanaBlockchain, create_solana_blockchain};
    
    #[tokio::test]
    async fn test_address_derivation() {
        let blockchain = create_solana_blockchain();
        
        // Test valid public key
        let public_key = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = blockchain.derive_solana_address(public_key);
        assert!(result.is_ok());
        
        let address = result.unwrap();
        assert_eq!(address.len() > 30, true);
        
        // Test invalid public key
        let invalid_key = "invalid";
        let result = blockchain.derive_solana_address(invalid_key);
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_address_validation() {
        let blockchain = create_solana_blockchain();
        
        // Test valid address
        assert!(blockchain.validate_address("11111111111111111111111111111111"));
        
        // Test invalid addresses
        assert!(!blockchain.validate_address(""));
        assert!(!blockchain.validate_address("invalid"));
        assert!(!blockchain.validate_address("1234"));
    }
    
    #[tokio::test]
    async fn test_transaction_building() {
        let blockchain = create_solana_blockchain();
        
        // Test SOL transfer transaction building
        let from = "11111111111111111111111111111111"; // System program (not a real sender)
        let to = "11111111111111111111111111111112";
        let amount = 1_000_000; // 0.001 SOL
        
        // This may fail if the RPC connection fails
        let result = blockchain.build_sol_transfer(from, to, amount).await;
        
        if result.is_err() {
            println!("Warning: SOL transaction build failed - this is expected in CI: {:?}", result.err());
            return; // Skip rest of test if we can't connect to RPC
        }
        
        let tx = result.unwrap();
        
        // Verify transaction data
        assert!(!tx.transaction_data.is_empty());
        assert!(!tx.message_hash.is_empty());
    }
}