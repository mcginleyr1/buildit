//! CLI command implementations.

pub mod pipelines;
pub mod run;
pub mod runs;

use anyhow::Result;

pub use run::run_local;

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
        Ok(pipeline) => {
            println!("Configuration is valid");
            println!("Pipeline: {}", pipeline.name);
            println!("Stages: {}", pipeline.stages.len());
            for stage in &pipeline.stages {
                let deps = if stage.needs.is_empty() {
                    String::new()
                } else {
                    format!(" (needs: {})", stage.needs.join(", "))
                };
                println!("  - {}{}", stage.name, deps);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    }
}
