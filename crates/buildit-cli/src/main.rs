//! BuildIt CLI tool.

use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "buildit")]
#[command(about = "BuildIt CI/CD CLI", long_about = None)]
struct Cli {
    /// API server URL
    #[arg(long, env = "BUILDIT_API_URL", default_value = "http://localhost:3000")]
    api_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with the API server
    Login {
        /// Authentication token
        #[arg(long)]
        token: Option<String>,
    },
    /// List pipelines
    Pipelines {
        #[command(subcommand)]
        command: PipelineCommands,
    },
    /// Manage pipeline runs
    Runs {
        #[command(subcommand)]
        command: RunCommands,
    },
    /// Deploy a service
    Deploy {
        /// Service name
        service: String,
        /// Target environment
        environment: String,
        /// Image to deploy
        #[arg(long)]
        image: Option<String>,
    },
    /// Rollback a deployment
    Rollback {
        /// Deployment ID or service name
        target: String,
    },
    /// Validate a pipeline configuration
    Validate {
        /// Path to the configuration file
        #[arg(default_value = "buildit.kdl")]
        path: String,
    },
}

#[derive(Subcommand)]
enum PipelineCommands {
    /// List all pipelines
    List {
        /// Filter by tenant
        #[arg(long)]
        tenant: Option<String>,
    },
    /// Trigger a pipeline run
    Trigger {
        /// Pipeline name or ID
        pipeline: String,
        /// Branch to build
        #[arg(long)]
        branch: Option<String>,
    },
}

#[derive(Subcommand)]
enum RunCommands {
    /// List recent runs
    List {
        /// Pipeline name or ID
        #[arg(long)]
        pipeline: Option<String>,
        /// Maximum number of runs to show
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    /// Show run details
    Show {
        /// Run ID
        id: String,
    },
    /// Stream logs from a run
    Logs {
        /// Run ID
        id: String,
        /// Follow logs in real-time
        #[arg(short, long)]
        follow: bool,
    },
    /// Cancel a running pipeline
    Cancel {
        /// Run ID
        id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login { token } => {
            commands::login(&cli.api_url, token).await?;
        }
        Commands::Pipelines { command } => match command {
            PipelineCommands::List { tenant } => {
                commands::pipelines::list(&cli.api_url, tenant).await?;
            }
            PipelineCommands::Trigger { pipeline, branch } => {
                commands::pipelines::trigger(&cli.api_url, &pipeline, branch).await?;
            }
        },
        Commands::Runs { command } => match command {
            RunCommands::List { pipeline, limit } => {
                commands::runs::list(&cli.api_url, pipeline, limit).await?;
            }
            RunCommands::Show { id } => {
                commands::runs::show(&cli.api_url, &id).await?;
            }
            RunCommands::Logs { id, follow } => {
                commands::runs::logs(&cli.api_url, &id, follow).await?;
            }
            RunCommands::Cancel { id } => {
                commands::runs::cancel(&cli.api_url, &id).await?;
            }
        },
        Commands::Deploy {
            service,
            environment,
            image,
        } => {
            commands::deploy(&cli.api_url, &service, &environment, image).await?;
        }
        Commands::Rollback { target } => {
            commands::rollback(&cli.api_url, &target).await?;
        }
        Commands::Validate { path } => {
            commands::validate(&path)?;
        }
    }

    Ok(())
}
