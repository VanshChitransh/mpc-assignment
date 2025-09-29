use crate::{Store, Asset, BalanceWithAsset, BalanceError};
use uuid::Uuid;

impl Store {
    /// Get user's SOL balance
    pub async fn get_sol_balance(&self, user_id: &Uuid) -> Result<i64, BalanceError> {
        let balance = sqlx::query_scalar!(
            r#"
            SELECT COALESCE(b.amount, 0) as amount
            FROM assets a
            LEFT JOIN balances b ON a.id = b.asset_id AND b.user_id = $1
            WHERE a.symbol = 'SOL'
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        Ok(balance.unwrap_or(0))
    }

    /// Get all token balances for a user (excluding SOL)
    pub async fn get_token_balances(&self, user_id: &Uuid) -> Result<Vec<BalanceWithAsset>, BalanceError> {
        let balances = sqlx::query!(
            r#"
            SELECT 
                COALESCE(b.amount, 0) as balance,
                a.mint_address as token_mint,
                a.symbol,
                a.decimals
            FROM assets a
            LEFT JOIN balances b ON a.id = b.asset_id AND b.user_id = $1
            WHERE a.symbol != 'SOL' AND (b.amount > 0 OR b.amount IS NULL)
            ORDER BY a.symbol
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        let result = balances
            .into_iter()
            .filter(|row| row.balance.unwrap_or(0) > 0) // Only return non-zero balances
            .map(|row| BalanceWithAsset {
                balance: row.balance.unwrap_or(0),
                token_mint: row.token_mint,
                symbol: row.symbol,
                decimals: row.decimals,
            })
            .collect();

        Ok(result)
    }

    /// Get balance for a specific asset
    pub async fn get_balance_for_asset(&self, user_id: &Uuid, asset_id: &Uuid) -> Result<i64, BalanceError> {
        let balance = sqlx::query_scalar!(
            "SELECT COALESCE(amount, 0)::bigint FROM balances WHERE user_id = $1 AND asset_id = $2",
            user_id,
            asset_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

       Ok(balance.unwrap_or(Some(0)).unwrap_or(0))
    }

    /// Update balance for a user and asset (used by indexer)
    pub async fn update_balance(&self, user_id: &Uuid, asset_id: &Uuid, new_amount: i64) -> Result<(), BalanceError> {
        // First verify the asset exists
        let asset_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM assets WHERE id = $1)",
            asset_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        if !asset_exists.unwrap_or(false) {
            return Err(BalanceError::AssetNotFound);
        }

        // Upsert the balance
        sqlx::query!(
            r#"
            INSERT INTO balances (id, user_id, asset_id, amount, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (user_id, asset_id)
            DO UPDATE SET amount = $4, updated_at = NOW()
            "#,
            Uuid::new_v4(),
            user_id,
            asset_id,
            new_amount
        )
        .execute(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Add to existing balance (positive amount adds, negative subtracts)
    pub async fn adjust_balance(&self, user_id: &Uuid, asset_id: &Uuid, amount_delta: i64) -> Result<i64, BalanceError> {
        // Get current balance
        let current_balance = self.get_balance_for_asset(user_id, asset_id).await?;
        let new_balance = current_balance + amount_delta;

        // Prevent negative balances
        if new_balance < 0 {
            return Err(BalanceError::InsufficientBalance);
        }

        // Update the balance
        self.update_balance(user_id, asset_id, new_balance).await?;
        
        Ok(new_balance)
    }

    /// Initialize zero balances for a new user (for supported assets)
    pub async fn initialize_user_balances(&self, user_id: &Uuid) -> Result<(), BalanceError> {
        // Get all supported assets
        let assets = sqlx::query!("SELECT id FROM assets")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        // Insert zero balance for each asset
        for asset in assets {
            sqlx::query!(
                r#"
                INSERT INTO balances (id, user_id, asset_id, amount, created_at, updated_at)
                VALUES ($1, $2, $3, 0, NOW(), NOW())
                ON CONFLICT (user_id, asset_id) DO NOTHING
                "#,
                Uuid::new_v4(),
                user_id,
                &asset.id
            )
            .execute(&self.pool)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Get asset by mint address
    pub async fn get_asset_by_mint(&self, mint_address: &str) -> Result<Asset, BalanceError> {
        let asset = sqlx::query_as!(
            Asset,
            "SELECT * FROM assets WHERE mint_address = $1",
            mint_address
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        asset.ok_or(BalanceError::AssetNotFound)
    }

    /// Get asset by symbol
    pub async fn get_asset_by_symbol(&self, symbol: &str) -> Result<Asset, BalanceError> {
        let asset = sqlx::query_as!(
            Asset,
            "SELECT * FROM assets WHERE symbol = $1",
            symbol
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        asset.ok_or(BalanceError::AssetNotFound)
    }

    /// Get all supported assets
    pub async fn get_all_assets(&self) -> Result<Vec<Asset>, BalanceError> {
        let assets = sqlx::query_as!(
            Asset,
            "SELECT * FROM assets ORDER BY symbol"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        Ok(assets)
    }

    /// Add a new supported asset
    pub async fn add_asset(&self, mint_address: String, decimals: i32, name: String, symbol: String, logo_url: Option<String>) -> Result<Asset, BalanceError> {
        let asset_id = Uuid::new_v4();
        
        let asset = sqlx::query_as!(
            Asset,
            r#"
            INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            RETURNING *
            "#,
            asset_id,
            mint_address,
            decimals,
            name,
            symbol,
            logo_url
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        Ok(asset)
    }

    /// Check if user has sufficient balance for a transaction
    pub async fn check_sufficient_balance(&self, user_id: &Uuid, asset_id: &Uuid, required_amount: i64) -> Result<bool, BalanceError> {
        let current_balance = self.get_balance_for_asset(user_id, asset_id).await?;
        Ok(current_balance >= required_amount)
    }

    /// Bulk update balances (used by indexer for efficient updates)
    pub async fn bulk_update_balances(&self, updates: Vec<(Uuid, Uuid, i64)>) -> Result<(), BalanceError> {
        let mut transaction = self.pool.begin().await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        for (user_id, asset_id, amount) in updates {
            sqlx::query!(
                r#"
                INSERT INTO balances (id, user_id, asset_id, amount, created_at, updated_at)
                VALUES ($1, $2, $3, $4, NOW(), NOW())
                ON CONFLICT (user_id, asset_id)
                DO UPDATE SET amount = $4, updated_at = NOW()
                "#,
                Uuid::new_v4(),
                &user_id,
                &asset_id,
                amount
            )
            .execute(&mut *transaction)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;
        }

        transaction.commit().await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_balance_operations() {
        if let Ok(database_url) = std::env::var("TEST_DATABASE_URL") {
            let pool = Store::new_pool(&database_url).await.unwrap();
            let store = Store::new(pool);
            
            // Test getting SOL balance for non-existent user
            let user_id = Uuid::new_v4();
            let balance = store.get_sol_balance(&user_id).await.unwrap();
            assert_eq!(balance, 0);
        }
    }
}