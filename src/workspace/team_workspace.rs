use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, Error, NoTls};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub team_id: String,
    pub name: String,
    pub database_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMember {
    pub user_id: String,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub file_path: String,
    pub content_hash: String,
    pub added_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct TeamWorkspace {
    config: WorkspaceConfig,
    client: Client,
}

impl TeamWorkspace {
    pub async fn new(config: WorkspaceConfig) -> Result<Self, Error> {
        let (client, connection) = tokio_postgres::connect(&config.database_url, NoTls).await?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(Self { config, client })
    }

    pub fn team_id(&self) -> &str {
        &self.config.team_id
    }

    pub async fn add_member(&self, user_id: &str, role: &str) -> Result<(), Error> {
        self.client
            .execute(
                "INSERT INTO workspace_members (team_id, user_id, role) VALUES ($1, $2, $3)",
                &[&self.config.team_id, &user_id, &role],
            )
            .await?;

        Ok(())
    }

    pub async fn remove_member(&self, user_id: &str) -> Result<(), Error> {
        self.client
            .execute(
                "DELETE FROM workspace_members WHERE team_id = $1 AND user_id = $2",
                &[&self.config.team_id, &user_id],
            )
            .await?;

        Ok(())
    }

    pub async fn list_members(&self) -> Result<Vec<WorkspaceMember>, Error> {
        let rows = self
            .client
            .query(
                "SELECT user_id, role, joined_at FROM workspace_members WHERE team_id = $1",
                &[&self.config.team_id],
            )
            .await?;

        let members = rows
            .iter()
            .map(|row| WorkspaceMember {
                user_id: row.get(0),
                role: row.get(1),
                joined_at: row.get(2),
            })
            .collect();

        Ok(members)
    }

    pub async fn add_knowledge(
        &self,
        file_path: &str,
        content_hash: &str,
        embedding: Option<Vec<f32>>,
        metadata: Option<serde_json::Value>,
    ) -> Result<String, Error> {
        let row = self
            .client
            .query_one(
                "INSERT INTO knowledge_base (team_id, file_path, content_hash, embedding, metadata)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id",
                &[
                    &self.config.team_id,
                    &file_path,
                    &content_hash,
                    &embedding,
                    &metadata,
                ],
            )
            .await?;

        let id: String = row.get(0);
        Ok(id)
    }

    pub async fn search_knowledge(&self, query: &str) -> Result<Vec<KnowledgeEntry>, Error> {
        let rows = self
            .client
            .query(
                "SELECT id, file_path, content_hash, user_id, created_at
             FROM knowledge_base
             WHERE team_id = $1 AND file_path ILIKE $2
             LIMIT 50",
                &[&self.config.team_id, &format!("%{}%", query)],
            )
            .await?;

        let entries = rows
            .iter()
            .map(|row| KnowledgeEntry {
                id: row.get(0),
                file_path: row.get(1),
                content_hash: row.get(2),
                added_by: row.get(3),
                created_at: row.get(4),
            })
            .collect();

        Ok(entries)
    }
}
