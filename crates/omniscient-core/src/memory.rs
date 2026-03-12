//! Memory system — short-term + long-term context management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

use crate::types::{EntityId, Finding};

/// Memory entry — a single piece of remembered information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: EntityId,
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f64, // 0.0 to 1.0
    pub access_count: u32,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    /// Short-term: current conversation/task context
    ShortTerm,
    /// Working: intermediate findings during research
    Working,
    /// Long-term: persisted knowledge across sessions
    LongTerm,
    /// Episodic: past research sessions and outcomes
    Episodic,
}

/// Memory system managing different memory tiers
pub struct Memory {
    /// Short-term memory — bounded, FIFO
    short_term: VecDeque<MemoryEntry>,
    short_term_capacity: usize,

    /// Working memory — current research findings
    working: Vec<Finding>,

    /// Long-term entries (will be backed by SurrealDB in production)
    long_term: Vec<MemoryEntry>,
}

impl Memory {
    pub fn new(short_term_capacity: usize) -> Self {
        Self {
            short_term: VecDeque::with_capacity(short_term_capacity),
            short_term_capacity,
            working: Vec::new(),
            long_term: Vec::new(),
        }
    }

    /// Add something to short-term memory
    pub fn remember_short(&mut self, content: String, importance: f64) {
        if self.short_term.len() >= self.short_term_capacity {
            // Evict least important
            if let Some(least_idx) = self
                .short_term
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.importance
                        .partial_cmp(&b.importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
            {
                self.short_term.remove(least_idx);
            }
        }

        self.short_term.push_back(MemoryEntry {
            id: Uuid::new_v4(),
            content,
            memory_type: MemoryType::ShortTerm,
            importance,
            access_count: 0,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            tags: Vec::new(),
        });
    }

    /// Add a research finding to working memory
    pub fn add_finding(&mut self, finding: Finding) {
        self.working.push(finding);
    }

    /// Get all working memory findings
    pub fn get_findings(&self) -> &[Finding] {
        &self.working
    }

    /// Clear working memory (after synthesis)
    pub fn clear_working(&mut self) {
        self.working.clear();
    }

    /// Promote important short-term memories to long-term
    pub fn consolidate(&mut self, importance_threshold: f64) {
        let promoted: Vec<MemoryEntry> = self
            .short_term
            .iter()
            .filter(|m| m.importance >= importance_threshold)
            .cloned()
            .map(|mut m| {
                m.memory_type = MemoryType::LongTerm;
                m
            })
            .collect();

        self.long_term.extend(promoted);
    }

    /// Search memory for relevant entries
    pub fn search(&self, query: &str) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&MemoryEntry> = self
            .short_term
            .iter()
            .chain(self.long_term.iter())
            .filter(|m| m.content.to_lowercase().contains(&query_lower))
            .collect();

        // Sort by importance (descending)
        results.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Get a context summary suitable for LLM prompts
    pub fn context_summary(&self, max_entries: usize) -> String {
        let mut entries: Vec<&MemoryEntry> = self
            .short_term
            .iter()
            .chain(self.long_term.iter())
            .collect();

        entries.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        entries
            .into_iter()
            .take(max_entries)
            .map(|e| format!("- {}", e.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get statistics about memory usage
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            short_term_count: self.short_term.len(),
            short_term_capacity: self.short_term_capacity,
            working_count: self.working.len(),
            long_term_count: self.long_term.len(),
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new(100)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub short_term_count: usize,
    pub short_term_capacity: usize,
    pub working_count: usize,
    pub long_term_count: usize,
}
