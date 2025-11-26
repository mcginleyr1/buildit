//! Run commands.

use anyhow::Result;

pub async fn list(_api_url: &str, pipeline: Option<String>, limit: u32) -> Result<()> {
    // TODO: Implement API call
    println!("Listing runs (pipeline: {:?}, limit: {})", pipeline, limit);
    println!("Not yet implemented");
    Ok(())
}

pub async fn show(_api_url: &str, id: &str) -> Result<()> {
    // TODO: Implement API call
    println!("Showing run {}", id);
    println!("Not yet implemented");
    Ok(())
}

pub async fn logs(_api_url: &str, id: &str, follow: bool) -> Result<()> {
    // TODO: Implement API call with WebSocket for follow
    println!("Logs for run {} (follow: {})", id, follow);
    println!("Not yet implemented");
    Ok(())
}

pub async fn cancel(_api_url: &str, id: &str) -> Result<()> {
    // TODO: Implement API call
    println!("Cancelling run {}", id);
    println!("Not yet implemented");
    Ok(())
}
