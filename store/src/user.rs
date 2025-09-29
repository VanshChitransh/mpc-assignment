use crate::{Store, User, CreateUserRequest, UserError, UserProfile};
use uuid::Uuid;
use chrono::Utc;

impl Store {

    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, UserError> {
 
        if !request.email.contains('@') || request.email.len() < 5 {
            return Err(UserError::InvalidInput("Invalid email format".to_string()));
        }


        if request.password.len() < 6 {
            return Err(UserError::InvalidInput("Password must be at least 6 characters".to_string()));
        }


        // let email = request.email.to_lowercase().trim().to_string();
        let email_lower = request.email.to_lowercase();
        let email_trimmed = email_lower.trim();


        let existing_user = sqlx::query!(
            "SELECT id FROM users WHERE email = $1",
            email_trimmed
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if existing_user.is_some() {
            return Err(UserError::UserExists(email_lower));
        }


        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
            .map_err(|e| UserError::PasswordHashFailed(e.to_string()))?;


        let user_id = Uuid::new_v4();
        let created_at = Utc::now();


        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, email, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $4)
            RETURNING id, email, password_hash, public_key, created_at, updated_at
            "#,
            user_id,
            email_trimmed,
            password_hash,
            created_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;


        Ok(user)
    }


    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<User, UserError> {

        let email_lower = email.to_lowercase();
        let email_trimmed = email_lower.trim();


        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, public_key, created_at, updated_at FROM users WHERE email = $1",
            email_trimmed
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::InvalidCredentials)?;


        let password_valid = bcrypt::verify(password, &user.password_hash)
            .map_err(|e| UserError::DatabaseError(format!("Password verification failed: {}", e)))?;

        if !password_valid {
            return Err(UserError::InvalidCredentials);
        }

        Ok(user)
    }


    pub async fn verify_user_password(&self, email: &str, password: &str) -> Result<Option<User>, UserError> {
        match self.authenticate_user(email, password).await {
            Ok(user) => Ok(Some(user)),
            Err(UserError::InvalidCredentials) => Ok(None),
            Err(e) => Err(e),
        }
    }


    pub async fn get_user_by_id(&self, user_id: &Uuid) -> Result<User, UserError> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, public_key, created_at, updated_at FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::UserNotFound)?;

        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, UserError> {
        let email_lower = email.to_lowercase();
        let email_trimmed = email_lower.trim();
        
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, public_key, created_at, updated_at FROM users WHERE email = $1",
            email_trimmed
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::UserNotFound)?;

        Ok(user)
    }

    pub async fn update_user_public_key(&self, user_id: &Uuid, public_key: &str) -> Result<(), UserError> {

        if public_key.len() < 32 || public_key.len() > 44 {
            return Err(UserError::InvalidInput("Invalid Solana public key format".to_string()));
        }

        let result = sqlx::query!(
            "UPDATE users SET public_key = $1, updated_at = NOW() WHERE id = $2",
            public_key,
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(UserError::UserNotFound);
        }

        Ok(())
    }


    pub async fn get_user_public_key(&self, user_id: &Uuid) -> Result<Option<String>, UserError> {
        let result = sqlx::query!(
            "SELECT public_key FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        match result {
            Some(row) => Ok(row.public_key),
            None => Err(UserError::UserNotFound),
        }
    }

    pub async fn update_user_password(&self, user_id: &Uuid, new_password: &str) -> Result<(), UserError> {
 
        if new_password.len() < 6 {
            return Err(UserError::InvalidInput("Password must be at least 6 characters".to_string()));
        }

        let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| UserError::PasswordHashFailed(e.to_string()))?;

        let result = sqlx::query!(
            "UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2",
            password_hash,
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(UserError::UserNotFound);
        }

        Ok(())
    }


    pub async fn delete_user(&self, user_id: &Uuid) -> Result<(), UserError> {
        let result = sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(UserError::UserNotFound);
        }

        Ok(())
    }

    pub async fn get_user_count(&self) -> Result<i64, UserError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM users"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }


    pub async fn user_has_keys(&self, user_id: &Uuid) -> Result<bool, UserError> {
        let public_key = self.get_user_public_key(user_id).await?;
        Ok(public_key.is_some())
    }

    pub async fn get_user_profile(&self, user_id: &Uuid) -> Result<UserProfile, UserError> {
        let user = self.get_user_by_id(user_id).await?;
        Ok(user.to_profile())
    }

    pub async fn list_users(&self, offset: i64, limit: i64) -> Result<Vec<User>, UserError> {
        let limit = limit.min(100);
        
        let users = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, public_key, created_at, updated_at FROM users ORDER BY created_at DESC OFFSET $1 LIMIT $2",
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        Ok(users)
    }

    pub async fn get_users_without_keys(&self) -> Result<Vec<User>, UserError> {
        let users = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, public_key, created_at, updated_at FROM users WHERE public_key IS NULL ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        Ok(users)
    }

    pub async fn get_user_stats(&self) -> Result<UserStats, UserError> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_users,
                COUNT(*) FILTER (WHERE public_key IS NOT NULL) as users_with_keys,
                COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '24 hours') as users_last_24h,
                COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '7 days') as users_last_week
            FROM users
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        Ok(UserStats {
            total_users: stats.total_users.unwrap_or(0),
            users_with_keys: stats.users_with_keys.unwrap_or(0),
            users_last_24h: stats.users_last_24h.unwrap_or(0),
            users_last_week: stats.users_last_week.unwrap_or(0),
        })
    }
}

#[derive(Debug)]
pub struct UserStats {
    pub total_users: i64,
    pub users_with_keys: i64,
    pub users_last_24h: i64,
    pub users_last_week: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    #[tokio::test]
    async fn test_user_operations() {
        if let Ok(database_url) = std::env::var("TEST_DATABASE_URL") {
            let pool = Store::new_pool(&database_url).await.unwrap();
            let store = Store::new(pool);

            let test_email = format!("test-{}@example.com", Uuid::new_v4());
            let create_request = CreateUserRequest {
                email: test_email.clone(),
                password: "testpassword123".to_string(),
            };
            
            let user = store.create_user(create_request).await.unwrap();
            assert_eq!(user.email, test_email.to_lowercase());
            
            let auth_user = store.authenticate_user(&test_email, "testpassword123").await.unwrap();
            assert_eq!(auth_user.id, user.id);

            let wrong_auth = store.authenticate_user(&test_email, "wrongpassword").await;
            assert!(matches!(wrong_auth, Err(UserError::InvalidCredentials)));
            
            let test_pubkey = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
            store.update_user_public_key(&user.id, test_pubkey).await.unwrap();
            
            let updated_pubkey = store.get_user_public_key(&user.id).await.unwrap();
            assert_eq!(updated_pubkey, Some(test_pubkey.to_string()));
            

            let has_keys = store.user_has_keys(&user.id).await.unwrap();
            assert!(has_keys);
            

            store.delete_user(&user.id).await.unwrap();
        }
    }
}