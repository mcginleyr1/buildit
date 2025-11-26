//! CLI command implementations.

pub mod pipelines;
pub mod runs;

use anyhow::Result;

pub async fn login(_api_url: &str, _token: Option<String>) -> Result<()> {
    // TODO: Implement login
    println!("Login not yet implemented");
    Ok(())
}

pub async fn deploy(
    _api_url: &str,
    service: &str,
    environment: &str,
    image: Option<String>,
) -> Result<()> {
    // TODO: Implement deploy
    println!(
        "Deploying {} to {} (image: {:?})",
        service, environment, image
    );
    println!("Deploy not yet implemented");
    Ok(())
}

pub async fn rollback(_api_url: &str, target: &str) -> Result<()> {
    // TODO: Implement rollback
    println!("Rolling back {}", target);
    println!("Rollback not yet implemented");
    Ok(())
}

pub fn validate(path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    match buildit_config::pipeline::parse_pipeline(&content) {
        Ok(_pipeline) => {
            println!("Configuration is valid");
            Ok(())
        }
        Err(e) => {
            println!("Configuration error: {}", e);
            std::process::exit(1);
        }
    }
}
