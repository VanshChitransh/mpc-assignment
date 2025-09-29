use sqlx::PgPool;
use tracing::{info, debug};
use serde_json::json;
use chrono::Utc;

use crate::yellowstone::{YellowstoneUpdate, AccountUpdate, TransactionUpdate};

#[derive(Debug, Clone)]
pub struct BalanceChange {
    pub address: String,
    pub old_balance: u64,
    pub new_balance: u64,
    pub slot: u64,
    pub transaction_signature: Option<String>,
    pub change_type: BalanceChangeType,
}

#[derive(Debug, Clone)]
pub enum BalanceChangeType {
    Transfer,
    Swap,
    StakeReward,
    Other(String),
}

pub struct TransactionProcessor {
    db_pool: PgPool,
}

impl TransactionProcessor {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn process_update(
        &self,
        update: YellowstoneUpdate,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match update {
            YellowstoneUpdate::Account(account_update) => {
                self.process_account_update(account_update).await
            }
            YellowstoneUpdate::Transaction(tx_update) => {
                self.process_transaction_update(tx_update).await
            }
        }
    }

    async fn process_account_update(
        &self,
        update: AccountUpdate,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing account update for address: {}", update.address);

        // Get the current balance from database
        let current_balance = self.get_current_balance(&update.address).await?;

        // Check if balance has changed
        if current_balance != update.lamports {
            info!(
                "Balance change detected for {}: {} -> {}",
                update.address, current_balance, update.lamports
            );

            let balance_change = BalanceChange {
                address: update.address.clone(),
                old_balance: current_balance,
                new_balance: update.lamports,
                slot: update.slot,
                transaction_signature: None,
                change_type: BalanceChangeType::Other("account_update".to_string()),
            };

            self.store_balance_change(&balance_change).await?;
            self.update_user_balance(&update.address, update.lamports).await?;
        }

        // Process SPL token data if present
        if !update.data.is_empty() && update.owner == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" {
            self.process_spl_token_account(&update).await?;
        }

        Ok(())
    }

    async fn process_transaction_update(
        &self,
        update: TransactionUpdate,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing transaction update: {}", update.signature);

        // Process balance changes from pre/post balances
        if update.accounts.len() == update.pre_balances.len() && 
           update.accounts.len() == update.post_balances.len() {
            
            for (i, account) in update.accounts.iter().enumerate() {
                let pre_balance = update.pre_balances[i];
                let post_balance = update.post_balances[i];

                if pre_balance != post_balance {
                    // Determine the type of transaction
                    let change_type = self.determine_transaction_type(&update.logs);

                    let balance_change = BalanceChange {
                        address: account.clone(),
                        old_balance: pre_balance,
                        new_balance: post_balance,
                        slot: update.slot,
                        transaction_signature: Some(update.signature.clone()),
                        change_type,
                    };

                    self.store_balance_change(&balance_change).await?;
                    self.update_user_balance(account, post_balance).await?;
                }
            }
        }

        // Store transaction details
        self.store_transaction_details(&update).await?;

        Ok(())
    }

    async fn process_spl_token_account(
        &self,
        update: &AccountUpdate,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing SPL token account: {}", update.address);

        // Parse SPL token account data
        if update.data.len() >= 72 { // Minimum SPL token account size
            let token_info = self.parse_spl_token_data(&update.data)?;
            
            // Update token balance in database
            self.update_token_balance(
                &update.address,
                &token_info.mint,
                token_info.amount,
                update.slot,
            ).await?;
        }

        Ok(())
    }

    fn determine_transaction_type(&self, logs: &[String]) -> BalanceChangeType {
        for log in logs {
            if log.contains("Program log: Instruction: Transfer") {
                return BalanceChangeType::Transfer;
            }
            if log.contains("Program log: Instruction: Swap") || log.contains("Jupiter") {
                return BalanceChangeType::Swap;
            }
            if log.contains("Stake") {
                return BalanceChangeType::StakeReward;
            }
        }
        BalanceChangeType::Other("unknown".to_string())
    }

    async fn get_current_balance(&self, address: &str) -> Result<u64, sqlx::Error> {
        let balance = sqlx::query_scalar!(
            "SELECT sol_balance FROM user_wallets WHERE address = $1",
            address
        )
        .fetch_optional(&self.db_pool)
        .await?;

        // Fixed the type handling
       Ok(balance.flatten().unwrap_or(0) as u64)
    }

    async fn store_balance_change(
        &self,
        change: &BalanceChange,
    ) -> Result<(), sqlx::Error> {
        let change_type_str = match &change.change_type {
            BalanceChangeType::Transfer => "Transfer",
            BalanceChangeType::Swap => "Swap",
            BalanceChangeType::StakeReward => "StakeReward",
            BalanceChangeType::Other(s) => s,
        };

        sqlx::query!(
            r#"
            INSERT INTO balance_changes 
            (address, old_balance, new_balance, slot, transaction_signature, change_type, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            change.address,
            change.old_balance as i64,
            change.new_balance as i64,
            change.slot as i64,
            change.transaction_signature,
            change_type_str,
            Utc::now()
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn update_user_balance(
        &self,
        address: &str,
        new_balance: u64,
    ) -> Result<(), sqlx::Error> {
        // Update wallet balance
        sqlx::query!(
            r#"
            INSERT INTO user_wallets (address, sol_balance, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (address) DO UPDATE SET
            sol_balance = $2,
            updated_at = $3
            "#,
            address,
            new_balance as i64,
            Utc::now()
        )
        .execute(&self.db_pool)
        .await?;

        // Update user total balance if user exists
        sqlx::query!(
            r#"
            UPDATE users 
            SET sol_balance = (
                SELECT COALESCE(SUM(sol_balance), 0) 
                FROM user_wallets 
                WHERE user_id = users.id
            ),
            updated_at = $1
            WHERE id IN (
                SELECT user_id FROM user_wallets WHERE address = $2 AND user_id IS NOT NULL
            )
            "#,
            Utc::now(),
            address
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn update_token_balance(
        &self,
        token_account: &str,
        mint: &str,
        amount: u64,
        slot: u64,
    ) -> Result<(), sqlx::Error> {
        // Insert or update token balance
        sqlx::query!(
            r#"
            INSERT INTO token_balances (token_account, mint, amount, slot, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (token_account, mint) 
            DO UPDATE SET 
                amount = $3,
                slot = $4,
                updated_at = $5
            WHERE token_balances.slot < $4
            "#,
            token_account,
            mint,
            amount as i64,
            slot as i64,
            Utc::now()
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn store_transaction_details(
        &self,
        update: &TransactionUpdate,
    ) -> Result<(), sqlx::Error> {
        let accounts_json = json!(update.accounts);
        let pre_balances_json = json!(update.pre_balances);
        let post_balances_json = json!(update.post_balances);
        let logs_json = json!(update.logs);

        sqlx::query!(
            r#"
            INSERT INTO transactions 
            (signature, slot, accounts, pre_balances, post_balances, logs, processed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (signature) DO NOTHING
            "#,
            update.signature,
            update.slot as i64,
            accounts_json,
            pre_balances_json,
            post_balances_json,
            logs_json,
            Utc::now()
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    fn parse_spl_token_data(&self, data: &[u8]) -> Result<SplTokenInfo, Box<dyn std::error::Error + Send + Sync>> {
        if data.len() < 72 {
            return Err("Invalid SPL token account data length".into());
        }

        // SPL Token Account Layout:
        // 0-32: mint (32 bytes)
        // 32-64: owner (32 bytes)
        // 64-72: amount (8 bytes, little-endian u64)

        let mint = bs58::encode(&data[0..32]).into_string();
        let amount = u64::from_le_bytes(
            data[64..72].try_into()
                .map_err(|_| "Failed to parse amount")?
        );

        Ok(SplTokenInfo {
            mint,
            amount,
        })
    }
}

#[derive(Debug)]
struct SplTokenInfo {
    mint: String,
    amount: u64,
}