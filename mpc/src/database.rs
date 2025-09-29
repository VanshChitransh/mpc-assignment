use sqlx::{PgPool, Row};
use uuid::Uuid;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyShare {
    pub user_id: Uuid,
    pub node_id: u32,
    pub key_share: String, // Encrypted/encoded key share
    pub public_key: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SigningSession {
    pub session_id: String,
    pub user_id: Uuid,
    pub transaction_hash: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn store_key_share(
        &self,
        user_id: Uuid,
        node_id: u32,
        key_share: &str,
        public_key: Option<&str>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO mpc_key_shares (user_id, node_id, key_share, public_key, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, node_id) 
            DO UPDATE SET 
                key_share = EXCLUDED.key_share,
                public_key = EXCLUDED.public_key,
                created_at = EXCLUDED.created_at
            "#,
            user_id,
            node_id as i32,
            key_share,
            public_key,
            Utc::now()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_key_share(&self, user_id: Uuid, node_id: u32) -> Result<Option<KeyShare>> {
        let row = sqlx::query!(
            "SELECT user_id, node_id, key_share, public_key, created_at FROM mpc_key_shares WHERE user_id = $1 AND node_id = $2",
            user_id,
            node_id as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(KeyShare {
                user_id: row.user_id,
                node_id: row.node_id as u32,
                key_share: row.key_share,
                public_key: row.public_key,
                created_at: row.created_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_public_key(&self, user_id: Uuid) -> Result<Option<String>> {
        let row = sqlx::query!(
            "SELECT public_key FROM mpc_key_shares WHERE user_id = $1 AND public_key IS NOT NULL LIMIT 1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| r.public_key))
    }

    pub async fn create_signing_session(
        &self,
        session_id: &str,
        user_id: Uuid,
        transaction_hash: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO mpc_signing_sessions (session_id, user_id, transaction_hash, status, created_at)
            VALUES ($1, $2, $3, 'pending', $4)
            "#,
            session_id,
            user_id,
            transaction_hash,
            Utc::now()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_signing_session_status(
        &self,
        session_id: &str,
        status: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE mpc_signing_sessions SET status = $1 WHERE session_id = $2",
            status,
            session_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_signing_session(&self, session_id: &str) -> Result<Option<SigningSession>> {
        let row = sqlx::query!(
            "SELECT session_id, user_id, transaction_hash, status, created_at FROM mpc_signing_sessions WHERE session_id = $1",
            session_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(SigningSession {
                session_id: row.session_id,
                user_id: row.user_id,
                transaction_hash: row.transaction_hash,
                status: row.status,
                created_at: row.created_at,
            }))
        } else {
            Ok(None)
        }
    }
}