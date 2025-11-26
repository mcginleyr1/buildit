//! Pipeline commands.

use anyhow::Result;

pub async fn list(_api_url: &str, tenant: Option<String>) -> Result<()> {
    // TODO: Implement API call
    println!("Listing pipelines (tenant: {:?})", tenant);
    println!("Not yet implemented");
    Ok(())
}

pub async fn trigger(_api_url: &str, pipeline: &str, branch: Option<String>) -> Result<()> {
    // TODO: Implement API call
    println!("Triggering pipeline {} (branch: {:?})", pipeline, branch);
    println!("Not yet implemented");
    Ok(())
}
