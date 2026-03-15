//! # Omniscient Research Pipelines
//!
//! Pre-built research workflows for deep research, profiling,
//! competitive analysis, and report generation.

pub mod deep_research;
pub mod report;
pub mod research_agent;
pub mod rig_agents;
pub mod dag_orchestrator;
pub mod evaluator;

pub use deep_research::DeepResearchPipeline;
pub use report::ReportGenerator;
pub use research_agent::ResearchAgent;
pub use dag_orchestrator::{RigOrchestrator, InvestigationState};
pub use evaluator::EvaluatorLoop;
