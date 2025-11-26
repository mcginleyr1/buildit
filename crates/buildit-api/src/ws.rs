//! WebSocket handling for real-time updates.

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::AppState;

/// WebSocket upgrade handler.
pub async fn ws_handler(ws: WebSocketUpgrade, State(_state): State<AppState>) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    info!("WebSocket connection established");

    while let Some(msg) = socket.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(cmd) = serde_json::from_str::<WsCommand>(&text) {
                    match cmd {
                        WsCommand::Subscribe { channel } => {
                            info!(channel = %channel, "Client subscribed");
                            let response = WsMessage::Subscribed {
                                channel: channel.clone(),
                            };
                            if let Ok(json) = serde_json::to_string(&response) {
                                let _ = socket.send(Message::Text(json.into())).await;
                            }
                        }
                        WsCommand::Unsubscribe { channel } => {
                            info!(channel = %channel, "Client unsubscribed");
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed");
                break;
            }
            Err(e) => {
                warn!(error = %e, "WebSocket error");
                break;
            }
            _ => {}
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
enum WsMessage {
    Subscribed { channel: String },
    RunUpdate { run_id: String, status: String },
    LogLine { run_id: String, line: String },
}
