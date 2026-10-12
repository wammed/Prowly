//! Omnipresent AI integration module.
//! Provides unified client abstraction for BYOK LLMs, context gathering, commit generation/review, and repository diagnostics.

pub mod ci;
pub mod client;
pub mod command;
pub mod commit;
pub mod context;
pub mod diagnostic;
pub mod merge;
pub mod pr;

pub use ci::{analyze_ci_failure, CiFailureAnalysis};
pub use client::{
    create_ai_client, AiClient, AiConfig, AiError, AiMessage, AiPrompt, AiProvider, AiResponse,
    AiRole, MockAiClient, OllamaClient,
};
pub use command::{translate_natural_language_to_git, CommandRiskLevel, SuggestedGitCommand};
pub use commit::{
    generate_commit_message, review_changes, truncate_diff, GeneratedCommit, DEFAULT_MAX_DIFF_CHARS,
};
pub use context::{AiContext, CiContext, ConflictContext, ContextProvider, PrContext};
pub use diagnostic::{diagnose_repository, RepoDiagnosis, RepoHealth};
pub use merge::smart_merge_conflict;
pub use pr::{generate_pr_draft, GeneratedPrDraft};
