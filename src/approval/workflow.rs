use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, Error, NoTls};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingKnowledge {
    pub file_path: String,
    pub content_hash: String,
    pub submitted_by: String,
    pub team_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ApprovalStatus::Pending => write!(f, "pending"),
            ApprovalStatus::Approved => write!(f, "approved"),
            ApprovalStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl From<String> for ApprovalStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "approved" => ApprovalStatus::Approved,
            "rejected" => ApprovalStatus::Rejected,
            _ => ApprovalStatus::Pending,
        }
    }
}

pub struct ApprovalWorkflow {
    client: Client,
}

impl ApprovalWorkflow {
    pub async fn new(database_url: &str) -> Result<Self, Error> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(Self { client })
    }

    pub async fn submit(&self, pending: PendingKnowledge) -> Result<String, Error> {
        let row = self.client.query_one(
            "INSERT INTO pending_knowledge (file_path, content_hash, submitted_by, team_id, status)
             VALUES ($1, $2, $3, $4, 'pending')
             RETURNING id",
            &[&pending.file_path, &pending.content_hash, &pending.submitted_by, &pending.team_id],
        ).await?;

        let id: String = row.get(0);
        Ok(id)
    }

    pub async fn approve(&self, id: &str, reviewer_id: &str) -> Result<(), Error> {
        self.client
            .execute(
                "UPDATE pending_knowledge
             SET status = 'approved', reviewed_by = $2, reviewed_at = NOW()
             WHERE id = $1",
                &[&id, &reviewer_id],
            )
            .await?;

        // Copy to knowledge_base
        self.client
            .execute(
                "INSERT INTO knowledge_base (team_id, file_path, content_hash, user_id)
             SELECT team_id, file_path, content_hash, submitted_by
             FROM pending_knowledge
             WHERE id = $1",
                &[&id],
            )
            .await?;

        Ok(())
    }

    pub async fn reject(&self, id: &str, reviewer_id: &str, reason: &str) -> Result<(), Error> {
        self.client
            .execute(
                "UPDATE pending_knowledge
             SET status = 'rejected', reviewed_by = $2, reviewed_at = NOW(), rejection_reason = $3
             WHERE id = $1",
                &[&id, &reviewer_id, &reason],
            )
            .await?;

        Ok(())
    }

    pub async fn get_status(&self, id: &str) -> Result<ApprovalStatus, Error> {
        let row = self
            .client
            .query_one("SELECT status FROM pending_knowledge WHERE id = $1", &[&id])
            .await?;

        let status_str: String = row.get(0);
        Ok(ApprovalStatus::from(status_str))
    }

    pub async fn list_pending(&self, team_id: &str) -> Result<Vec<PendingKnowledge>, Error> {
        let rows = self
            .client
            .query(
                "SELECT file_path, content_hash, submitted_by, team_id
             FROM pending_knowledge
             WHERE team_id = $1 AND status = 'pending'
             ORDER BY created_at DESC",
                &[&team_id],
            )
            .await?;

        let pending = rows
            .iter()
            .map(|row| PendingKnowledge {
                file_path: row.get(0),
                content_hash: row.get(1),
                submitted_by: row.get(2),
                team_id: row.get(3),
            })
            .collect();

        Ok(pending)
    }
}
