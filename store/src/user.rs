use crate::{Store, User, CreateUserRequest, UserError};
use uuid::Uuid;
use chrono::Utc;

impl Store {
    /// Create a new user with validation and password hashing
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, UserError> {
        // Validate email format
        if !request.email.contains('@') {
            return Err(UserError::InvalidInput("Invalid email format".to_string()));
        }

        // Validate password length
        if request.password.len() < 6 {
            return Err(UserError::InvalidInput("Password must be at least 6 characters".to_string()));
        }

        // Check if user already exists
        let existing_user = sqlx::query!(
            "SELECT id FROM users WHERE email = $1",
            request.email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if existing_user.is_some() {
            return Err(UserError::UserExists);
        }

        // Hash the password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
            .map_err(|e| UserError::DatabaseError(format!("Password hashing failed: {}", e)))?;

        // Generate user ID and timestamp
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();

        // Insert user into database
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, email, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $4)
            RETURNING *
            "#,
            user_id,
            request.email,
            password_hash,
            created_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    /// Authenticate user with email and password
    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<User, UserError> {
        // Get user by email
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::InvalidCredentials)?;

        // Verify password
        let password_valid = bcrypt::verify(password, &user.password_hash)
            .map_err(|e| UserError::DatabaseError(format!("Password verification failed: {}", e)))?;

        if !password_valid {
            return Err(UserError::InvalidCredentials);
        }

        Ok(user)
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &Uuid) -> Result<User, UserError> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::UserNotFound)?;

        Ok(user)
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<User, UserError> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::UserNotFound)?;

        Ok(user)
    }

    /// Update user's Solana public key (called after MPC key generation)
    pub async fn update_user_public_key(&self, user_id: &Uuid, public_key: &str) -> Result<(), UserError> {
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

    /// Get user's public key
    pub async fn get_user_public_key(&self, user_id: &Uuid) -> Result<Option<String>, UserError> {
        let public_key = sqlx::query_scalar!(
            "SELECT public_key FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        match public_key {
            Some(Some(key)) => Ok(Some(key)),
            Some(None) => Ok(None),
            None => Err(UserError::UserNotFound),
        }
    }

    /// Update user's password
    pub async fn update_user_password(&self, user_id: &Uuid, new_password: &str) -> Result<(), UserError> {
        // Validate password length
        if new_password.len() < 6 {
            return Err(UserError::InvalidInput("Password must be at least 6 characters".to_string()));
        }

        // Hash the new password
        let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| UserError::DatabaseError(format!("Password hashing failed: {}", e)))?;

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

    /// Delete user (for admin purposes - be careful!)
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

    /// Get user count (for monitoring)
    pub async fn get_user_count(&self) -> Result<i64, UserError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM users"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }

    /// Check if user has completed MPC key generation
    pub async fn user_has_keys(&self, user_id: &Uuid) -> Result<bool, UserError> {
        let public_key = self.get_user_public_key(user_id).await?;
        Ok(public_key.is_some())
    }

    /// List users for admin purposes (with pagination)
    pub async fn list_users(&self, offset: i64, limit: i64) -> Result<Vec<User>, UserError> {
        let users = sqlx::query_as!(
            User,
            "SELECT * FROM users ORDER BY created_at DESC OFFSET $1 LIMIT $2",
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        Ok(users)
    }

    /// Get users without public keys (need MPC key generation)
    pub async fn get_users_without_keys(&self) -> Result<Vec<User>, UserError> {
        let users = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE public_key IS NULL ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_creation_and_auth() {
        if let Ok(database_url) = std::env::var("TEST_DATABASE_URL") {
            let pool = Store::new_pool(&database_url).await.unwrap();
            let store = Store::new(pool);

            let email = "test@example.com";
            let password = "testpassword123";

            // Test user creation
            let create_request = CreateUserRequest {
                email: email.to_string(),
                password: password.to_string(),
            };

            let user = store.create_user(create_request).await.unwrap();
            assert_eq!(user.email, email);
            assert!(user.public_key.is_none());

            // Test authentication
            let auth_user = store.authenticate_user(email, password).await.unwrap();
            assert_eq!(auth_user.id, user.id);

            // Test wrong password
            let wrong_auth = store.authenticate_user(email, "wrongpassword").await;
            assert!(matches!(wrong_auth, Err(UserError::InvalidCredentials)));

            // Test updating public key
            let public_key = "SomePublicKeyString123";
            let user_uuid = Uuid::parse_str(&user.id).unwrap();
            store.update_user_public_key(&user_uuid, public_key).await.unwrap();    

            let updated_key = store.get_user_public_key(&user_uuid).await.unwrap();
            assert_eq!(updated_key, Some(public_key.to_string()));

            // Test user has keys
            let has_keys = store.user_has_keys(&user_uuid).await.unwrap();
            assert!(has_keys);

            // Clean up
            store.delete_user(&user_uuid).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_user_validation() {
        if let Ok(database_url) = std::env::var("TEST_DATABASE_URL") {
            let pool = Store::new_pool(&database_url).await.unwrap();
            let store = Store::new(pool);

            // Test invalid email
            let invalid_email = CreateUserRequest {
                email: "invalid-email".to_string(),
                password: "validpassword123".to_string(),
            };
            let result = store.create_user(invalid_email).await;
            assert!(matches!(result, Err(UserError::InvalidInput(_))));

            // Test short password
            let short_password = CreateUserRequest {
                email: "valid@example.com".to_string(),
                password: "123".to_string(),
            };
            let result = store.create_user(short_password).await;
            assert!(matches!(result, Err(UserError::InvalidInput(_))));
        }
    }
}