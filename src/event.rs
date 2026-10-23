//! Asynchronous event loop bridging crossterm events (keyboard, mouse, resize) and background tasks.

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, MouseEvent};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::ai::{
    CiFailureAnalysis, GeneratedCommit, GeneratedPrDraft, RepoDiagnosis, SuggestedGitCommand,
};
use crate::github::{AlertItem, IssueItem, PullRequestItem, RepoMetadata, WorkflowRunItem};

/// Core events processed by the Prowly application.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    MouseMoved(u16, u16),
    Resize(u16, u16),
    Tick,
    StatusMessage(String),
    GitFetchCompleted(Result<String, String>),
    GitPullCompleted(Result<String, String>),
    GitPushCompleted(Result<String, String>),
    GitHubRepoMetadataLoaded(RepoMetadata),
    GitHubPrsLoaded(Vec<PullRequestItem>),
    GitHubIssuesLoaded(Vec<IssueItem>),
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
    AiAssistantChunkReceived(String),
    GitHubWorkflowRerunResult(Result<String, String>),
}

/// Asynchronous event listener sending crossterm and timer events over a channel.
pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<AppEvent>,
    sender: mpsc::UnboundedSender<AppEvent>,
    pause_sender: tokio::sync::watch::Sender<bool>,
}

impl EventHandler {
    /// Initialize event listener with a tick interval.
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (pause_sender, mut pause_receiver) = tokio::sync::watch::channel(false);
        let event_sender = sender.clone();

        tokio::spawn(async move {
            let mut tick_timer = tokio::time::interval(tick_rate);
            let mut last_mouse_cell: Option<(u16, u16)> = None;

            loop {
                // If paused, wait until resumed before attaching EventStream to stdin
                if *pause_receiver.borrow() {
                    if pause_receiver.changed().await.is_err() {
                        break;
                    }
                    continue;
                }

                let mut reader = EventStream::new();
                'active: loop {
                    tokio::select! {
                        res = pause_receiver.changed() => {
                            if res.is_err() {
                                return;
                            }
                            if *pause_receiver.borrow() {
                                // Drop reader immediately so child processes can inherit stdin cleanly
                                break 'active;
                            }
                        }
                        _ = tick_timer.tick() => {
                            if event_sender.send(AppEvent::Tick).is_err() {
                                return;
                            }
                        }
                        maybe_event = reader.next() => {
                            match maybe_event {
                                Some(Ok(event)) => match event {
                                    CrosstermEvent::Key(key) => {
                                        if event_sender.send(AppEvent::Key(key)).is_err() {
                                            return;
                                        }
                                    }
                                    CrosstermEvent::Mouse(mouse) => {
                                        if matches!(mouse.kind, crossterm::event::MouseEventKind::Moved) {
                                            let cell = (mouse.column, mouse.row);
                                            if last_mouse_cell != Some(cell) {
                                                last_mouse_cell = Some(cell);
                                                if event_sender.send(AppEvent::MouseMoved(cell.0, cell.1)).is_err() {
                                                    return;
                                                }
                                            }
                                        } else {
                                            last_mouse_cell = Some((mouse.column, mouse.row));
                                            if event_sender.send(AppEvent::Mouse(mouse)).is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    CrosstermEvent::Resize(w, h)
                                        if event_sender.send(AppEvent::Resize(w, h)).is_err() =>
                                    {
                                        return;
                                    }
                                    _ => {}
                                },
                                Some(Err(_)) => {}
                                None => return,
                            }
                        }
                    }
                }
            }
        });

        Self {
            receiver,
            sender,
            pause_sender,
        }
    }

    /// Retrieve the event channel sender for background worker tasks.
    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.sender.clone()
    }

    /// Pause the event listener and drop the stdin stream to prevent input contention.
    pub fn pause(&self) {
        let _ = self.pause_sender.send(true);
    }

    /// Resume the event listener and re-attach the stdin stream.
    pub fn resume(&self) {
        let _ = self.pause_sender.send(false);
    }

    /// Discard all pending events in the queue (e.g. after returning from an external editor).
    pub fn flush(&mut self) {
        while self.receiver.try_recv().is_ok() {}
    }

    /// Await the next event in the queue.
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.receiver.recv().await
    }

    /// Attempt to fetch an event immediately without waiting.
    pub fn try_recv(&mut self) -> Result<AppEvent, tokio::sync::mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }
}
