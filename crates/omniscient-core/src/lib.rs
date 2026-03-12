//! # Omniscient Core
//!
//! The brain of the Omniscient deep research agent.
//! Provides the orchestration engine, agent traits, tool system,
//! memory management, and configuration.

pub mod agent;
pub mod config;
pub mod error;
pub mod memory;
pub mod orchestrator;
pub mod task;
pub mod tools;
pub mod types;

pub use agent::{Agent, AgentContext};
pub use config::OmniscientConfig;
pub use error::{OmniscientError, Result};
pub use memory::Memory;
pub use orchestrator::Orchestrator;
pub use task::{ResearchTask, TaskStatus};
pub use tools::{Tool, ToolInput, ToolOutput, ToolRegistry};
