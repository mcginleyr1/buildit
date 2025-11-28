//! Webhook endpoints for Git providers.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::AppState;
use crate::error::ApiError;
use buildit_core::ResourceId;
use buildit_core::repository::{GitProvider, PushEvent};
use buildit_db::{PipelineRepo, RepositoryRepo};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/github", post(github_webhook))
        .route("/github/{repo_id}", post(github_webhook_with_id))
}

/// Handle GitHub webhook events.
async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    process_github_webhook(state, headers, body, None).await
}

/// Handle GitHub webhook events with explicit repo ID.
async fn github_webhook_with_id(
    State(state): State<AppState>,
    Path(repo_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    process_github_webhook(state, headers, body, Some(repo_id)).await
}

async fn process_github_webhook(
    state: AppState,
    headers: HeaderMap,
    body: Bytes,
    repo_id: Option<uuid::Uuid>,
) -> Result<StatusCode, ApiError> {
    // Get event type
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Get signature
    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Parse payload
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Extract repository info from payload
    let repo_full_name = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(|n| n.as_str());

    info!(
        event = %event_type,
        repo = ?repo_full_name,
        "Received GitHub webhook"
    );

    // Find the repository
    let repository = if let Some(id) = repo_id {
        Some(
            state
                .repository_repo
                .get_by_id(ResourceId::from_uuid(id))
                .await?,
        )
    } else if let Some(full_name) = repo_full_name {
        // Try to find by provider ID
        state
            .repository_repo
            .get_by_provider_id(GitProvider::Github, full_name)
            .await?
    } else {
        None
    };

    // Store the webhook event
    let headers_json = serde_json::json!({
        "event": event_type,
        "delivery": headers.get("X-GitHub-Delivery").and_then(|v| v.to_str().ok()),
    });

    let webhook_event = state
        .repository_repo
        .create_webhook_event(
            repository.as_ref().map(|r| ResourceId::from_uuid(r.id)),
            GitProvider::Github,
            event_type,
            payload.clone(),
            headers_json,
            signature.as_deref(),
        )
        .await?;

    // Validate signature if repository has webhook secret
    if let Some(ref repo) = repository {
        if let Some(ref secret) = repo.webhook_secret {
            let is_valid = verify_github_signature(secret, &body, signature.as_deref());
            state
                .repository_repo
                .update_webhook_signature_valid(ResourceId::from_uuid(webhook_event.id), is_valid)
                .await?;

            if !is_valid {
                warn!(repo_id = %repo.id, "Invalid webhook signature");
                return Ok(StatusCode::UNAUTHORIZED);
            }
        }
    }

    // Process the event
    match event_type {
        "push" => {
            if let Some(push_event) = PushEvent::from_github_payload(&payload) {
                handle_push_event(&state, repository.as_ref(), push_event).await?;
            }
        }
        "pull_request" => {
            // TODO: Handle PR events
            info!("Pull request event received (not yet implemented)");
        }
        "ping" => {
            info!("Ping event received - webhook is configured correctly");
        }
        _ => {
            info!(event = %event_type, "Unhandled event type");
        }
    }

    // Mark as processed
    state
        .repository_repo
        .mark_webhook_processed(ResourceId::from_uuid(webhook_event.id), None)
        .await?;

    Ok(StatusCode::OK)
}

/// Handle a push event by triggering matching pipelines.
async fn handle_push_event(
    state: &AppState,
    repository: Option<&buildit_core::repository::Repository>,
    push_event: PushEvent,
) -> Result<(), ApiError> {
    let Some(repo) = repository else {
        warn!(
            repo = %push_event.repository_full_name,
            "Push event for unknown repository"
        );
        return Ok(());
    };

    info!(
        repo = %repo.full_name,
        branch = ?push_event.branch,
        sha = %push_event.after,
        "Processing push event"
    );

    // Find pipelines linked to this repository
    let pipelines = state
        .pipeline_repo
        .list_by_repository(ResourceId::from_uuid(repo.id))
        .await?;

    if pipelines.is_empty() {
        info!(repo = %repo.full_name, "No pipelines configured for this repository");
        return Ok(());
    }

    info!(
        repo = %repo.full_name,
        pipeline_count = pipelines.len(),
        "Found pipelines to trigger"
    );

    // Build git info for the run
    let git_info = serde_json::json!({
        "sha": push_event.after,
        "short_sha": &push_event.after[..7.min(push_event.after.len())],
        "branch": push_event.branch,
        "ref": push_event.r#ref,
        "message": push_event.head_commit.as_ref().map(|c| &c.message),
        "author": push_event.head_commit.as_ref().map(|c| &c.author),
        "repository": push_event.repository_full_name,
    });

    // Build trigger info
    let trigger_info = serde_json::json!({
        "kind": "push",
        "actor": push_event.pusher,
        "ref": push_event.r#ref,
    });

    // Trigger each pipeline
    for pipeline in pipelines {
        // Check if pipeline has trigger configuration
        let config = &pipeline.config;
        let triggers = config.get("triggers").and_then(|t| t.as_array());

        // Check if this push matches any trigger conditions
        let should_trigger = match triggers {
            Some(triggers) => {
                triggers.iter().any(|trigger| {
                    match trigger.get("type").and_then(|t| t.as_str()) {
                        Some("push") => {
                            // Check branch filter if present
                            if let Some(branches) =
                                trigger.get("branches").and_then(|b| b.as_array())
                            {
                                let branch_patterns: Vec<&str> =
                                    branches.iter().filter_map(|b| b.as_str()).collect();

                                if let Some(ref branch) = push_event.branch {
                                    matches_branch_pattern(branch, &branch_patterns)
                                } else {
                                    false
                                }
                            } else {
                                // No branch filter means trigger on all branches
                                true
                            }
                        }
                        _ => false,
                    }
                })
            }
            None => {
                // No triggers configured - default to triggering on all pushes to default branch
                push_event.branch.as_deref() == Some(&repo.default_branch)
            }
        };

        if !should_trigger {
            info!(
                pipeline = %pipeline.name,
                branch = ?push_event.branch,
                "Pipeline trigger conditions not met, skipping"
            );
            continue;
        }

        // Create a pipeline run
        match state
            .pipeline_repo
            .create_run(
                ResourceId::from_uuid(pipeline.id),
                trigger_info.clone(),
                git_info.clone(),
            )
            .await
        {
            Ok(run) => {
                info!(
                    pipeline = %pipeline.name,
                    run_id = %run.id,
                    run_number = run.number,
                    "Created pipeline run from webhook"
                );

                // TODO: Queue the run for execution via the orchestrator
                // For now, just mark it as queued (which is the default)
            }
            Err(e) => {
                error!(
                    pipeline = %pipeline.name,
                    error = %e,
                    "Failed to create pipeline run"
                );
            }
        }
    }

    Ok(())
}

/// Check if a branch name matches any of the given patterns.
/// Supports simple glob patterns with '*' wildcard.
fn matches_branch_pattern(branch: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| {
        if pattern.contains('*') {
            // Simple glob matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let (prefix, suffix) = (parts[0], parts[1]);
                branch.starts_with(prefix) && branch.ends_with(suffix)
            } else if pattern.starts_with('*') {
                branch.ends_with(&pattern[1..])
            } else if pattern.ends_with('*') {
                branch.starts_with(&pattern[..pattern.len() - 1])
            } else {
                // Complex glob - fall back to exact match
                branch == *pattern
            }
        } else {
            branch == *pattern
        }
    })
}

/// Verify GitHub webhook signature.
fn verify_github_signature(secret: &str, body: &[u8], signature: Option<&str>) -> bool {
    let Some(signature) = signature else {
        return false;
    };

    // Signature format: "sha256=<hex>"
    let Some(sig_hex) = signature.strip_prefix("sha256=") else {
        return false;
    };

    let Ok(sig_bytes) = hex::decode(sig_hex) else {
        return false;
    };

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take any size key");
    mac.update(body);

    mac.verify_slice(&sig_bytes).is_ok()
}
