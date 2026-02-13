use chrono::{DateTime, Utc};
use crate::core::error::StorageError;
use crate::core::session::Session;
use sqlx::SqlitePool;

pub struct SessionRepo {
    pool: SqlitePool,
}

impl SessionRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, session: &Session) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO sessions (id, title, message_count, prompt_tokens, \
             completion_tokens, cost, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session.id)
        .bind(&session.title)
        .bind(session.message_count as i64)
        .bind(session.prompt_tokens as i64)
        .bind(session.completion_tokens as i64)
        .bind(session.cost)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<Session, StorageError> {
        let row: (String, String, i64, i64, i64, f64, String, String) = sqlx::query_as(
            "SELECT id, title, message_count, prompt_tokens, \
             completion_tokens, cost, created_at, updated_at \
             FROM sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?
        .ok_or_else(|| StorageError::NotFound(format!("session {id}")))?;

        Ok(row_to_session(row))
    }

    pub async fn list(&self) -> Result<Vec<Session>, StorageError> {
        let rows: Vec<(String, String, i64, i64, i64, f64, String, String)> = sqlx::query_as(
            "SELECT id, title, message_count, prompt_tokens, \
             completion_tokens, cost, created_at, updated_at \
             FROM sessions ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(row_to_session).collect())
    }

    pub async fn update(&self, session: &Session) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE sessions SET title = ?, message_count = ?, \
             prompt_tokens = ?, completion_tokens = ?, cost = ?, \
             updated_at = ? WHERE id = ?",
        )
        .bind(&session.title)
        .bind(session.message_count as i64)
        .bind(session.prompt_tokens as i64)
        .bind(session.completion_tokens as i64)
        .bind(session.cost)
        .bind(Utc::now().to_rfc3339())
        .bind(&session.id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }
}

fn row_to_session(row: (String, String, i64, i64, i64, f64, String, String)) -> Session {
    Session {
        id: row.0,
        title: row.1,
        message_count: row.2 as u64,
        prompt_tokens: row.3 as u64,
        completion_tokens: row.4 as u64,
        cost: row.5,
        created_at: DateTime::parse_from_rfc3339(&row.6)
            .unwrap_or_default()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&row.7)
            .unwrap_or_default()
            .with_timezone(&Utc),
    }
}
