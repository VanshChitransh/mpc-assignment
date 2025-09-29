use crate::{Store, Asset, BalanceWithAsset, BalanceError};
use uuid::Uuid;

impl Store {

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

    pub async fn update_balance(&self, user_id: &Uuid, asset_id: &Uuid, new_amount: i64) -> Result<(), BalanceError> {

        let asset_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM assets WHERE id = $1)",
            asset_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        if !asset_exists.unwrap_or(false) {
            return Err(BalanceError::AssetNotFound(*asset_id));
        }

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

    pub async fn adjust_balance(&self, user_id: &Uuid, asset_id: &Uuid, amount_delta: i64) -> Result<i64, BalanceError> {

        let current_balance = self.get_balance_for_asset(user_id, asset_id).await?;
        let new_balance = current_balance + amount_delta;

        if new_balance < 0 {
            return Err(BalanceError::InsufficientBalance {
                required: amount_delta.abs(),
                available: current_balance,
            });
        }

        self.update_balance(user_id, asset_id, new_balance).await?;
        
        Ok(new_balance)
    }

    pub async fn initialize_user_balances(&self, user_id: &Uuid) -> Result<(), BalanceError> {

        let assets = sqlx::query!("SELECT id FROM assets")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

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

    pub async fn get_or_create_asset(&self, mint_address: &str, decimals: i32, name: Option<String>, symbol: Option<String>) -> Result<Asset, BalanceError> {
        if let Ok(asset) = self.get_asset_by_mint(mint_address).await {
            return Ok(asset);
        }
        let name = name.unwrap_or_else(|| format!("Unknown Token ({})", &mint_address[0..6]));
        let symbol = symbol.unwrap_or_else(|| mint_address[0..6].to_string());
        
        self.add_asset(
            mint_address.to_string(),
            decimals,
            name,
            symbol,
            None,
        ).await
    }

    pub async fn get_asset_by_mint(&self, mint_address: &str) -> Result<Asset, BalanceError> {
        let asset = sqlx::query_as!(
            Asset,
            "SELECT * FROM assets WHERE mint_address = $1",
            mint_address
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        asset.ok_or(BalanceError::AssetNotFoundByMint(mint_address.to_string()))
    }

    pub async fn get_asset_by_symbol(&self, symbol: &str) -> Result<Asset, BalanceError> {
        let asset = sqlx::query_as!(
            Asset,
            "SELECT * FROM assets WHERE symbol = $1",
            symbol
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        asset.ok_or(BalanceError::AssetNotFoundBySymbol(symbol.to_string()))
    }

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

    pub async fn add_asset(&self, mint_address: String, decimals: i32, name: String, symbol: String, logo_url: Option<String>) -> Result<Asset, BalanceError> {
        let asset_id = Uuid::new_v4();
        
        let asset = sqlx::query_as!(
            Asset,
            r#"
            INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            ON CONFLICT (mint_address) DO UPDATE 
            SET decimals = EXCLUDED.decimals,
                name = EXCLUDED.name,
                symbol = EXCLUDED.symbol,
                logo_url = COALESCE(EXCLUDED.logo_url, assets.logo_url),
                updated_at = NOW()
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

    pub async fn check_sufficient_balance(&self, user_id: &Uuid, asset_id: &Uuid, required_amount: i64) -> Result<bool, BalanceError> {
        let current_balance = self.get_balance_for_asset(user_id, asset_id).await?;
        Ok(current_balance >= required_amount)
    }

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
   pub async fn update_balance_by_mint(&self, user_id: &Uuid, mint_address: &str, new_amount: i64) -> Result<(), BalanceError> {
        let asset = self.get_asset_by_mint(mint_address).await?;
        self.update_balance(user_id, &asset.id, new_amount).await
    }

    pub async fn get_balance_by_mint(&self, user_id: &Uuid, mint_address: &str) -> Result<i64, BalanceError> {
        let asset = self.get_asset_by_mint(mint_address).await?;
        self.get_balance_for_asset(user_id, &asset.id).await
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

            store.initialize_default_assets().await.unwrap();

            let user_id = Uuid::new_v4();
            let balance = store.get_sol_balance(&user_id).await.unwrap();
            assert_eq!(balance, 0);

            store.initialize_user_balances(&user_id).await.unwrap();

            let sol_asset = store.get_asset_by_symbol("SOL").await.unwrap();
            store.update_balance(&user_id, &sol_asset.id, 1_000_000_000).await.unwrap();
            
            let updated_balance = store.get_sol_balance(&user_id).await.unwrap();
            assert_eq!(updated_balance, 1_000_000_000);
 
            let new_balance = store.adjust_balance(&user_id, &sol_asset.id, -500_000_000).await.unwrap();
            assert_eq!(new_balance, 500_000_000);

            let result = store.adjust_balance(&user_id, &sol_asset.id, -600_000_000).await;
            assert!(result.is_err());
        }
    }
}