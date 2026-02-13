use chrono::{DateTime, Utc};
use crate::core::error::StorageError;
use crate::core::message::{ContentPart, Message, MessageRole, TokenUsage};
use crate::core::model::ModelId;
use sqlx::SqlitePool;

pub struct MessageRepo {
    pool: SqlitePool,
}

impl MessageRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, msg: &Message) -> Result<(), StorageError> {
        let parts_json =
            serde_json::to_string(&msg.parts).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let usage_json = msg
            .token_usage
            .as_ref()
            .map(|u| serde_json::to_string(u).unwrap_or_default());
        let role_str = serde_json::to_string(&msg.role)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        sqlx::query(
            "INSERT INTO messages (id, session_id, role, parts_json, model_id, \
             usage_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&msg.id)
        .bind(&msg.session_id)
        .bind(&role_str)
        .bind(&parts_json)
        .bind(msg.model_id.as_ref().map(|m| m.to_string()))
        .bind(&usage_json)
        .bind(msg.created_at.to_rfc3339())
        .bind(msg.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn update(&self, msg: &Message) -> Result<(), StorageError> {
        let parts_json =
            serde_json::to_string(&msg.parts).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let usage_json = msg
            .token_usage
            .as_ref()
            .map(|u| serde_json::to_string(u).unwrap_or_default());

        sqlx::query(
            "UPDATE messages SET parts_json = ?, usage_json = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&parts_json)
        .bind(&usage_json)
        .bind(Utc::now().to_rfc3339())
        .bind(&msg.id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn list(&self, session_id: &str) -> Result<Vec<Message>, StorageError> {
        let rows: Vec<(String, String, String, String, Option<String>, Option<String>, String, String)> =
            sqlx::query_as(
                "SELECT id, session_id, role, parts_json, model_id, \
                 usage_json, created_at, updated_at \
                 FROM messages WHERE session_id = ? ORDER BY created_at ASC",
            )
            .bind(session_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        rows.into_iter().map(row_to_message).collect()
    }

    pub async fn delete_session_messages(&self, session_id: &str) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }
}

fn row_to_message(
    row: (String, String, String, String, Option<String>, Option<String>, String, String),
) -> Result<Message, StorageError> {
    let role: MessageRole = serde_json::from_str(&format!("\"{}\"", row.2))
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let parts: Vec<ContentPart> =
        serde_json::from_str(&row.3).map_err(|e| StorageError::Serialization(e.to_string()))?;
    let token_usage: Option<TokenUsage> = row
        .5
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let model_id = row.4.map(ModelId);

    Ok(Message {
        id: row.0,
        session_id: row.1,
        role,
        parts,
        model_id,
        token_usage,
        created_at: DateTime::parse_from_rfc3339(&row.6)
            .unwrap_or_default()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&row.7)
            .unwrap_or_default()
            .with_timezone(&Utc),
    })
}
