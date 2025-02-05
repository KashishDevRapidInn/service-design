use deadpool_redis::{Config, Pool, Connection};
use anyhow::Context;
use errors::{AuthError, CustomError};
use std::sync::Arc;
use redis::AsyncCommands;  // we still need `redis` for the commands but don't manually instantiate the client
                           
#[derive(Clone)]
pub struct RedisService {
    pub pool: Arc<Pool>,
}

impl RedisService {
    pub async fn new(redis_url: String) -> RedisService {
        let config = Config::from_url(redis_url);
        let pool = config.create_pool().expect("Failed to create Redis pool");
        RedisService {
            pool: Arc::new(pool),
        }
    }

    pub async fn get_connection(&self) -> Result<Connection, std::io::Error> {
        self.pool.get().await.map_err(|e| {
            eprintln!("Failed to create Redis session store: {:?}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Redis connection failed")
        })
    }

    pub async fn get_session(&self, session_id: String) -> Result<Option<String>, CustomError> {
        let mut con = self.get_connection()
            .await
            .context("Failed to get Redis connection")?;
        let user_id: Option<String> = con.hget(session_id, "user_id").await.ok();
        Ok(user_id)
    }

    pub async fn set_session(&self, session_id: &str, user_id: &str) -> Result<(), CustomError> {
        let mut con: Connection = self.get_connection()
            .await
            .context("Failed to get Redis connection")?;
        con.hset(session_id, "user_id", user_id)
            .await
            .context("Failed to set session")?;
        con.expire(session_id, 3600)
            .await
            .context("Failed to set session expiry")?; // Expire after 1 hour
        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), CustomError> {
        let mut con = self.get_connection()
            .await
            .context("Failed to get Redis connection")?;
        con.del(session_id)
            .await
            .context("Failed to delete session")?;
        Ok(())
    }

    pub async fn get_user_from_session(&self, sid: &String) -> Result<String, CustomError> {
        let user_id_str = self.get_session(sid.to_string()).await?;

        match user_id_str {
            Some(id_user) => Ok(id_user),
            None => {
                Err(AuthError::InvalidSession(anyhow::anyhow!("Failed to get user for session")).into())
            }
        }
    }
}
