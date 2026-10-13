//! Asynchronous event loop bridging crossterm events (keyboard, mouse, resize) and background tasks.

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, MouseEvent};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::ai::{
    CiFailureAnalysis, GeneratedCommit, GeneratedPrDraft, RepoDiagnosis, SuggestedGitCommand,
};
use crate::github::{AlertItem, PullRequestItem, RepoMetadata, WorkflowRunItem};

/// Core events processed by the Prowly application.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    StatusMessage(String),
    GitHubRepoMetadataLoaded(RepoMetadata),
    GitHubPrsLoaded(Vec<PullRequestItem>),
    GitHubRunsLoaded(Vec<WorkflowRunItem>),
    GitHubAlertsLoaded(Vec<AlertItem>),
    AiCommitGenerated(Result<GeneratedCommit, String>),
    AiReviewCompleted(Result<String, String>),
    AiDiagnosticCompleted(Result<RepoDiagnosis, String>),
    AiPrDraftGenerated(Result<GeneratedPrDraft, String>),
    AiCiAnalysisCompleted(Result<CiFailureAnalysis, String>),
    AiCommandSuggested(Result<SuggestedGitCommand, String>),
    AiMergeResolved(Result<String, String>),
    AiAssistantResponseReceived(Result<(String, Option<String>), String>),
    GitHubWorkflowRerunResult(Result<String, String>),
}

/// Asynchronous event listener sending crossterm and timer events over a channel.
pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<AppEvent>,
    sender: mpsc::UnboundedSender<AppEvent>,
}

impl EventHandler {
    /// Initialize event listener with a tick interval.
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let event_sender = sender.clone();

        tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_timer = tokio::time::interval(tick_rate);

            loop {
                tokio::select! {
                    _ = tick_timer.tick() => {
                        if event_sender.send(AppEvent::Tick).is_err() {
                            break;
                        }
                    }
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(event)) => match event {
                                CrosstermEvent::Key(key) => {
                                    if event_sender.send(AppEvent::Key(key)).is_err() {
                                        break;
                                    }
                                }
                                CrosstermEvent::Mouse(mouse) => {
                                    if event_sender.send(AppEvent::Mouse(mouse)).is_err() {
                                        break;
                                    }
                                }
                                CrosstermEvent::Resize(w, h)
                                    if event_sender.send(AppEvent::Resize(w, h)).is_err() =>
                                {
                                    break;
                                }
                                _ => {}
                            },
                            Some(Err(_)) => {}
                            None => break,
                        }
                    }
                }
            }
        });

        Self { receiver, sender }
    }

    /// Retrieve the event channel sender for background worker tasks.
    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.sender.clone()
    }

    /// Await the next event in the queue.
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.receiver.recv().await
    }
}
