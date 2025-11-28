//! Variable interpolation for pipeline configurations.
//!
//! Supports variables like:
//! - `${git.sha}` - Full git commit SHA
//! - `${git.short_sha}` - Short (7 char) git commit SHA
//! - `${git.branch}` - Current branch name
//! - `${git.tag}` - Git tag (if applicable)
//! - `${git.ref}` - Git ref (branch or tag)
//! - `${git.message}` - Commit message (first line)
//! - `${git.author}` - Commit author name
//! - `${git.author_email}` - Commit author email
//! - `${pipeline.name}` - Pipeline name
//! - `${pipeline.id}` - Pipeline ID
//! - `${run.id}` - Run ID
//! - `${run.number}` - Run number
//! - `${stage.name}` - Current stage name
//! - `${env.VAR_NAME}` - Environment variable
//! - `${secrets.SECRET_NAME}` - Secret value
//! - `${timestamp}` - Unix timestamp
//! - `${date}` - ISO date (YYYY-MM-DD)
//! - `${datetime}` - ISO datetime

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Variable context containing all available variables for interpolation.
#[derive(Debug, Clone, Default)]
pub struct VariableContext {
    /// Git-related variables
    pub git: GitContext,
    /// Pipeline-related variables
    pub pipeline: PipelineContext,
    /// Run-related variables
    pub run: RunContext,
    /// Stage-related variables
    pub stage: StageContext,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Secrets (will be masked in logs)
    pub secrets: HashMap<String, String>,
    /// Custom variables defined by user
    pub custom: HashMap<String, String>,
}

/// Git context for variable interpolation.
#[derive(Debug, Clone, Default)]
pub struct GitContext {
    pub sha: String,
    pub short_sha: String,
    pub branch: String,
    pub tag: Option<String>,
    pub ref_name: String,
    pub message: String,
    pub author: String,
    pub author_email: String,
}

/// Pipeline context for variable interpolation.
#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    pub id: String,
    pub name: String,
    pub repository: String,
}

/// Run context for variable interpolation.
#[derive(Debug, Clone, Default)]
pub struct RunContext {
    pub id: String,
    pub number: u32,
    pub trigger: String,
}

/// Stage context for variable interpolation.
#[derive(Debug, Clone, Default)]
pub struct StageContext {
    pub name: String,
    pub index: usize,
}

// Regex for matching ${...} variables
static VAR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)?)\}").unwrap()
});

impl VariableContext {
    /// Create a new empty variable context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with git information populated from a repository path.
    pub fn from_git_repo(repo_path: &str) -> Self {
        let mut ctx = Self::new();
        ctx.populate_git_from_repo(repo_path);
        ctx
    }

    /// Populate git context from environment variables (CI environment).
    pub fn populate_git_from_env(&mut self) {
        // Common CI environment variables
        self.git.sha = std::env::var("BUILDIT_COMMIT_SHA")
            .or_else(|_| std::env::var("GIT_COMMIT"))
            .or_else(|_| std::env::var("GITHUB_SHA"))
            .or_else(|_| std::env::var("CI_COMMIT_SHA"))
            .unwrap_or_default();

        if !self.git.sha.is_empty() {
            self.git.short_sha = self.git.sha.chars().take(7).collect();
        }

        self.git.branch = std::env::var("BUILDIT_BRANCH")
            .or_else(|_| std::env::var("GIT_BRANCH"))
            .or_else(|_| std::env::var("GITHUB_REF_NAME"))
            .or_else(|_| std::env::var("CI_COMMIT_BRANCH"))
            .unwrap_or_default();

        self.git.tag = std::env::var("BUILDIT_TAG")
            .or_else(|_| std::env::var("CI_COMMIT_TAG"))
            .ok()
            .or_else(|| {
                // GitHub Actions: check if ref type is tag
                std::env::var("GITHUB_REF_TYPE")
                    .ok()
                    .filter(|t| t == "tag")
                    .and_then(|_| std::env::var("GITHUB_REF_NAME").ok())
            });

        self.git.ref_name = self
            .git
            .tag
            .clone()
            .unwrap_or_else(|| self.git.branch.clone());

        self.git.message = std::env::var("BUILDIT_COMMIT_MESSAGE")
            .or_else(|_| std::env::var("CI_COMMIT_MESSAGE"))
            .unwrap_or_default();

        self.git.author = std::env::var("BUILDIT_COMMIT_AUTHOR")
            .or_else(|_| std::env::var("CI_COMMIT_AUTHOR"))
            .unwrap_or_default();

        self.git.author_email = std::env::var("BUILDIT_COMMIT_AUTHOR_EMAIL").unwrap_or_default();
    }

    /// Populate git context by running git commands in a repo path.
    pub fn populate_git_from_repo(&mut self, repo_path: &str) {
        use std::process::Command;

        let run_git = |args: &[&str]| -> Option<String> {
            Command::new("git")
                .args(args)
                .current_dir(repo_path)
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        };

        if let Some(sha) = run_git(&["rev-parse", "HEAD"]) {
            self.git.sha = sha.clone();
            self.git.short_sha = sha.chars().take(7).collect();
        }

        if let Some(branch) = run_git(&["rev-parse", "--abbrev-ref", "HEAD"]) {
            if branch != "HEAD" {
                self.git.branch = branch.clone();
                self.git.ref_name = branch;
            }
        }

        // Try to get tag at HEAD
        if let Some(tag) = run_git(&["describe", "--tags", "--exact-match", "HEAD"]) {
            self.git.tag = Some(tag.clone());
            self.git.ref_name = tag;
        }

        if let Some(message) = run_git(&["log", "-1", "--format=%s"]) {
            self.git.message = message;
        }

        if let Some(author) = run_git(&["log", "-1", "--format=%an"]) {
            self.git.author = author;
        }

        if let Some(email) = run_git(&["log", "-1", "--format=%ae"]) {
            self.git.author_email = email;
        }
    }

    /// Populate environment variables from the current process environment.
    pub fn populate_env(&mut self) {
        for (key, value) in std::env::vars() {
            self.env.insert(key, value);
        }
    }

    /// Add a custom variable.
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        self.custom.insert(name.to_string(), value.into());
    }

    /// Resolve a variable name to its value.
    pub fn resolve(&self, var_name: &str) -> Option<String> {
        // Check for nested access (e.g., "git.sha")
        let parts: Vec<&str> = var_name.split('.').collect();

        match parts.as_slice() {
            ["git", "sha"] => Some(self.git.sha.clone()),
            ["git", "short_sha"] => Some(self.git.short_sha.clone()),
            ["git", "branch"] => Some(self.git.branch.clone()),
            ["git", "tag"] => self.git.tag.clone(),
            ["git", "ref"] => Some(self.git.ref_name.clone()),
            ["git", "message"] => Some(self.git.message.clone()),
            ["git", "author"] => Some(self.git.author.clone()),
            ["git", "author_email"] => Some(self.git.author_email.clone()),

            ["pipeline", "id"] => Some(self.pipeline.id.clone()),
            ["pipeline", "name"] => Some(self.pipeline.name.clone()),
            ["pipeline", "repository"] => Some(self.pipeline.repository.clone()),

            ["run", "id"] => Some(self.run.id.clone()),
            ["run", "number"] => Some(self.run.number.to_string()),
            ["run", "trigger"] => Some(self.run.trigger.clone()),

            ["stage", "name"] => Some(self.stage.name.clone()),
            ["stage", "index"] => Some(self.stage.index.to_string()),

            ["env", name] => self.env.get(*name).cloned(),
            ["secrets", name] => self.secrets.get(*name).cloned(),

            ["timestamp"] => Some(chrono::Utc::now().timestamp().to_string()),
            ["date"] => Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            ["datetime"] => Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),

            // Single-part names check custom variables
            [name] => self.custom.get(*name).cloned(),

            _ => None,
        }
    }

    /// Interpolate all variables in a string.
    /// Variables are specified as `${var_name}` or `${namespace.var_name}`.
    pub fn interpolate(&self, input: &str) -> String {
        VAR_REGEX
            .replace_all(input, |caps: &regex::Captures| {
                let var_name = &caps[1];
                self.resolve(var_name)
                    .unwrap_or_else(|| format!("${{{}}}", var_name))
            })
            .to_string()
    }

    /// Interpolate variables in a list of strings.
    pub fn interpolate_vec(&self, inputs: &[String]) -> Vec<String> {
        inputs.iter().map(|s| self.interpolate(s)).collect()
    }

    /// Interpolate variables in a HashMap.
    pub fn interpolate_map(&self, map: &HashMap<String, String>) -> HashMap<String, String> {
        map.iter()
            .map(|(k, v)| (k.clone(), self.interpolate(v)))
            .collect()
    }

    /// Get a list of all secret variable names used in a string (for masking).
    pub fn find_secrets_in_string(&self, input: &str) -> Vec<String> {
        VAR_REGEX
            .captures_iter(input)
            .filter_map(|caps| {
                let var_name = &caps[1];
                if var_name.starts_with("secrets.") {
                    Some(var_name.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all secret values that should be masked in logs.
    pub fn get_secret_values(&self) -> Vec<&str> {
        self.secrets.values().map(|s| s.as_str()).collect()
    }
}

/// Builder for creating VariableContext.
pub struct VariableContextBuilder {
    ctx: VariableContext,
}

impl VariableContextBuilder {
    pub fn new() -> Self {
        Self {
            ctx: VariableContext::new(),
        }
    }

    pub fn with_git_sha(mut self, sha: impl Into<String>) -> Self {
        let sha = sha.into();
        self.ctx.git.short_sha = sha.chars().take(7).collect();
        self.ctx.git.sha = sha;
        self
    }

    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        let branch = branch.into();
        if self.ctx.git.ref_name.is_empty() {
            self.ctx.git.ref_name = branch.clone();
        }
        self.ctx.git.branch = branch;
        self
    }

    pub fn with_git_tag(mut self, tag: impl Into<String>) -> Self {
        let tag = tag.into();
        self.ctx.git.ref_name = tag.clone();
        self.ctx.git.tag = Some(tag);
        self
    }

    pub fn with_pipeline(mut self, id: impl Into<String>, name: impl Into<String>) -> Self {
        self.ctx.pipeline.id = id.into();
        self.ctx.pipeline.name = name.into();
        self
    }

    pub fn with_run(mut self, id: impl Into<String>, number: u32) -> Self {
        self.ctx.run.id = id.into();
        self.ctx.run.number = number;
        self
    }

    pub fn with_stage(mut self, name: impl Into<String>, index: usize) -> Self {
        self.ctx.stage.name = name.into();
        self.ctx.stage.index = index;
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.ctx.env.insert(key.into(), value.into());
        self
    }

    pub fn with_secret(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.ctx.secrets.insert(key.into(), value.into());
        self
    }

    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.ctx.custom.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> VariableContext {
        self.ctx
    }
}

impl Default for VariableContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interpolation() {
        let ctx = VariableContextBuilder::new()
            .with_git_sha("abc1234567890")
            .with_git_branch("main")
            .build();

        let result = ctx.interpolate("Commit ${git.sha} on ${git.branch}");
        assert_eq!(result, "Commit abc1234567890 on main");
    }

    #[test]
    fn test_short_sha() {
        let ctx = VariableContextBuilder::new()
            .with_git_sha("abc1234567890def")
            .build();

        let result = ctx.interpolate("Short: ${git.short_sha}");
        assert_eq!(result, "Short: abc1234");
    }

    #[test]
    fn test_env_variables() {
        let ctx = VariableContextBuilder::new()
            .with_env("MY_VAR", "hello")
            .with_env("ANOTHER", "world")
            .build();

        let result = ctx.interpolate("${env.MY_VAR} ${env.ANOTHER}!");
        assert_eq!(result, "hello world!");
    }

    #[test]
    fn test_secrets() {
        let ctx = VariableContextBuilder::new()
            .with_secret("API_KEY", "super-secret-key")
            .build();

        let result = ctx.interpolate("Key: ${secrets.API_KEY}");
        assert_eq!(result, "Key: super-secret-key");
    }

    #[test]
    fn test_unknown_variable_preserved() {
        let ctx = VariableContext::new();
        let result = ctx.interpolate("Unknown: ${unknown.var}");
        assert_eq!(result, "Unknown: ${unknown.var}");
    }

    #[test]
    fn test_pipeline_and_run_context() {
        let ctx = VariableContextBuilder::new()
            .with_pipeline("pipeline-123", "my-pipeline")
            .with_run("run-456", 42)
            .with_stage("build", 1)
            .build();

        let result =
            ctx.interpolate("Pipeline ${pipeline.name} run #${run.number} stage ${stage.name}");
        assert_eq!(result, "Pipeline my-pipeline run #42 stage build");
    }

    #[test]
    fn test_interpolate_vec() {
        let ctx = VariableContextBuilder::new()
            .with_git_branch("develop")
            .build();

        let inputs = vec![
            "echo ${git.branch}".to_string(),
            "deploy to ${git.branch}".to_string(),
        ];
        let results = ctx.interpolate_vec(&inputs);
        assert_eq!(results[0], "echo develop");
        assert_eq!(results[1], "deploy to develop");
    }

    #[test]
    fn test_custom_variables() {
        let mut ctx = VariableContext::new();
        ctx.set("version", "1.2.3");
        ctx.set("app_name", "myapp");

        let result = ctx.interpolate("${app_name} v${version}");
        assert_eq!(result, "myapp v1.2.3");
    }

    #[test]
    fn test_timestamp_variables() {
        let ctx = VariableContext::new();

        let result = ctx.interpolate("${date}");
        // Should be in YYYY-MM-DD format
        assert!(result.len() == 10);
        assert!(result.contains('-'));
    }

    #[test]
    fn test_find_secrets() {
        let ctx = VariableContext::new();
        let secrets = ctx.find_secrets_in_string(
            "Using ${secrets.API_KEY} and ${secrets.DB_PASSWORD} with ${git.sha}",
        );
        assert_eq!(secrets.len(), 2);
        assert!(secrets.contains(&"secrets.API_KEY".to_string()));
        assert!(secrets.contains(&"secrets.DB_PASSWORD".to_string()));
    }

    #[test]
    fn test_nested_braces() {
        let ctx = VariableContextBuilder::new().with_git_sha("abc123").build();

        // Make sure we don't mess up JSON or other nested braces
        let result = ctx.interpolate(r#"{"sha": "${git.sha}"}"#);
        assert_eq!(result, r#"{"sha": "abc123"}"#);
    }
}
