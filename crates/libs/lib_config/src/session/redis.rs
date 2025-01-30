use deadpool_redis::{Config, Pool, Connection};
use errors::AuthError;
use std::sync::Arc;
use redis::AsyncCommands;  // we still need `redis` for the commands but don't manually instantiate the client
use uuid::Uuid;
#[derive(Clone)]
pub struct RedisService {
    pub pool: Arc<Pool>,
}

impl RedisService {
    // Initialize the Redis pool
    pub async fn new(redis_url: String) -> RedisService {
        let config = Config::from_url(redis_url);
        let pool = config.create_pool().expect("Failed to create Redis pool");
        RedisService {
            pool: Arc::new(pool),
        }
    }

    // Get a Redis connection from the pool
    pub async fn get_connection(&self) -> Result<Connection, std::io::Error> {
        self.pool.get().await.map_err(|e| {
            eprintln!("Failed to create Redis session store: {:?}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Redis connection failed")
        })
    }

    // Example method to get session from Redis
    pub async fn get_session(&self, session_id: String) -> Result<Option<String>, deadpool_redis::redis::RedisError> {
        let mut con = self.get_connection().await.unwrap();
        let user_id: Option<String> = con.get(session_id).await.ok();
        Ok(user_id)
    }

    // Example method to set session in Redis   
    pub async fn set_session(&self, session_id: &str, user_id: &str) -> Result<(), deadpool_redis::redis::RedisError> {
        let mut con: Connection = self.get_connection().await.unwrap();
        con.set(session_id, user_id).await?;
        con.expire(session_id, 3600).await?; // Expire after 1 hour
        Ok(())
    }

    // Example method to delete session
    pub async fn delete_session(&self, session_id: &str) -> Result<(), deadpool_redis::redis::RedisError> {
        let mut con = self.get_connection().await.unwrap();
        con.del(session_id).await?;
        Ok(())
    }

    // Helper function for getting uid from sid
    pub async fn get_user_from_session(&self, sid: &String) -> Result<String, AuthError> {
        let user_id_str = self.get_session(sid.to_string()).await.map_err(|_| {
            AuthError::SessionAuthenticationError(
                "User not found".to_string(),
            )
        })?;

        match user_id_str {
            Some(id_user) => Ok(id_user),
            None => {
                Err(AuthError::SessionAuthenticationError("User not found".to_string()).into())
            }
        }
    }
}
