//! WebSocket handling for real-time updates.

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::AppState;

/// Broadcast event sent to WebSocket clients.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BroadcastEvent {
    RunUpdate {
        run_id: String,
        status: String,
    },
    StageUpdate {
        run_id: String,
        stage_name: String,
        status: String,
        duration: Option<String>,
    },
    LogLine {
        run_id: String,
        stage_name: String,
        content: String,
        stream: String,
    },
}

/// Broadcaster for WebSocket events.
#[derive(Clone)]
pub struct Broadcaster {
    tx: broadcast::Sender<BroadcastEvent>,
}

impl Broadcaster {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    /// Send an event to all connected WebSocket clients.
    pub fn send(&self, event: BroadcastEvent) {
        // Ignore errors if no receivers
        let _ = self.tx.send(event);
    }

    /// Subscribe to receive events.
    pub fn subscribe(&self) -> broadcast::Receiver<BroadcastEvent> {
        self.tx.subscribe()
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    let broadcaster = state.broadcaster.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, broadcaster))
}

async fn handle_socket(socket: WebSocket, broadcaster: Arc<Broadcaster>) {
    info!("WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();
    let mut subscriptions: HashSet<String> = HashSet::new();
    let mut broadcast_rx = broadcaster.subscribe();

    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<WsCommand>(&text) {
                            match cmd {
                                WsCommand::Subscribe { channel } => {
                                    info!(channel = %channel, "Client subscribed");
                                    subscriptions.insert(channel.clone());
                                    let response = WsResponse::Subscribed { channel };
                                    if let Ok(json) = serde_json::to_string(&response) {
                                        let _ = sender.send(Message::Text(json.into())).await;
                                    }
                                }
                                WsCommand::Unsubscribe { channel } => {
                                    info!(channel = %channel, "Client unsubscribed");
                                    subscriptions.remove(&channel);
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "WebSocket error");
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast events
            event = broadcast_rx.recv() => {
                match event {
                    Ok(event) => {
                        // Check if client is subscribed to this event's channel
                        let channel = match &event {
                            BroadcastEvent::RunUpdate { run_id, .. } => format!("run:{}", run_id),
                            BroadcastEvent::StageUpdate { run_id, .. } => format!("run:{}", run_id),
                            BroadcastEvent::LogLine { run_id, .. } => format!("run:{}", run_id),
                        };

                        if subscriptions.contains(&channel) || subscriptions.contains("*") {
                            if let Ok(json) = serde_json::to_string(&event) {
                                if sender.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Client is too slow, skip some messages
                        warn!("WebSocket client lagging, skipping messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsCommand {
    Subscribe { channel: String },
    Unsubscribe { channel: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsResponse {
    Subscribed { channel: String },
}
