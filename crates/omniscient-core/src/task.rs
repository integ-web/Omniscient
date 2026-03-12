//! Task management — research tasks and their lifecycle

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{EntityId, ResearchDepth, TaskCategory};

/// A research task — the unit of work in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchTask {
    pub id: EntityId,
    pub query: String,
    pub task_type: TaskType,
    pub category: Option<TaskCategory>,
    pub depth: ResearchDepth,
    pub status: TaskStatus,
    pub subtasks: Vec<SubTask>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: f64, // 0.0 to 1.0
    pub metadata: serde_json::Value,
}

impl ResearchTask {
    pub fn new(query: impl Into<String>, depth: ResearchDepth) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            query: query.into(),
            task_type: TaskType::General,
            category: None,
            depth,
            status: TaskStatus::Pending,
            subtasks: Vec::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            progress: 0.0,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_type(mut self, task_type: TaskType) -> Self {
        self.task_type = task_type;
        self
    }

    pub fn update_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
        self.updated_at = Utc::now();
    }

    pub fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.progress = 1.0;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = TaskStatus::Failed(error);
        self.updated_at = Utc::now();
    }
}

/// Type of research task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    General,
    CompanyProfile,
    PersonProfile,
    CompetitiveAnalysis,
    LiteratureReview,
    ReverseEngineering,
    FactCheck,
    TrendAnalysis,
}

/// Status of a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Planning,
    Running,
    Synthesizing,
    Completed,
    Failed(String),
    Cancelled,
}

/// A subtask within a research task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: EntityId,
    pub parent_id: EntityId,
    pub description: String,
    pub tool: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub depends_on: Vec<EntityId>,
}

impl SubTask {
    pub fn new(parent_id: EntityId, description: impl Into<String>, tool: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_id,
            description: description.into(),
            tool: tool.into(),
            status: TaskStatus::Pending,
            result: None,
            depends_on: Vec::new(),
        }
    }
}
