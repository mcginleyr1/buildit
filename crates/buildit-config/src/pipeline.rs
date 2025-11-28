//! Pipeline configuration parsing.

use crate::{ConfigError, ConfigResult};
use buildit_core::ResourceId;
use buildit_core::pipeline::{CacheConfig, Pipeline, Stage, StageAction, StageCondition, Trigger};
use kdl::{KdlDocument, KdlNode};
use std::collections::HashMap;

/// Parse a pipeline configuration from KDL text.
pub fn parse_pipeline(kdl: &str) -> ConfigResult<Pipeline> {
    let doc: KdlDocument = kdl.parse()?;

    let mut name = String::new();
    let mut triggers = Vec::new();
    let mut stages = Vec::new();
    let mut caches = Vec::new();
    let mut env = HashMap::new();

    for node in doc.nodes() {
        match node.name().value() {
            "pipeline" => {
                name = get_first_string_arg(node)
                    .ok_or_else(|| ConfigError::MissingField("pipeline name".to_string()))?;
            }
            "on" => {
                triggers.push(parse_trigger(node)?);
            }
            "stage" => {
                stages.push(parse_stage(node)?);
            }
            "cache" => {
                caches.push(parse_cache(node)?);
            }
            "env" => {
                if let Some(children) = node.children() {
                    for child in children.nodes() {
                        let key = child.name().value().to_string();
                        if let Some(val) = get_first_string_arg(child) {
                            env.insert(key, val);
                        }
                    }
                }
            }
            _ => {} // Ignore unknown nodes
        }
    }

    if name.is_empty() {
        return Err(ConfigError::MissingField("pipeline name".to_string()));
    }

    // Validate DAG - check for missing dependencies
    let stage_names: Vec<&str> = stages.iter().map(|s| s.name.as_str()).collect();
    for stage in &stages {
        for dep in &stage.needs {
            if !stage_names.contains(&dep.as_str()) {
                return Err(ConfigError::InvalidReference(format!(
                    "stage '{}' depends on unknown stage '{}'",
                    stage.name, dep
                )));
            }
        }
    }

    // Check for cycles
    if let Err(cycle) = detect_cycle(&stages) {
        return Err(ConfigError::CycleDetected(cycle));
    }

    Ok(Pipeline {
        id: ResourceId::new(),
        name,
        tenant_id: ResourceId::new(), // Will be set by caller
        repository: String::new(),    // Will be set by caller
        triggers,
        stages,
        env,
        caches,
    })
}

fn parse_trigger(node: &KdlNode) -> ConfigResult<Trigger> {
    let trigger_type = get_first_string_arg(node).unwrap_or_default();

    match trigger_type.as_str() {
        "push" => {
            let branches = get_string_list_prop(node, "branches");
            let paths = get_string_list_prop(node, "paths");
            Ok(Trigger::Push {
                branches: if branches.is_empty() {
                    vec!["*".to_string()]
                } else {
                    branches
                },
                paths: if paths.is_empty() { None } else { Some(paths) },
            })
        }
        "pull_request" => {
            let branches = get_string_list_prop(node, "branches");
            Ok(Trigger::PullRequest {
                branches: if branches.is_empty() {
                    None
                } else {
                    Some(branches)
                },
            })
        }
        "tag" => {
            let pattern = get_string_prop(node, "pattern");
            Ok(Trigger::Tag { pattern })
        }
        "schedule" => {
            let cron = get_string_prop(node, "cron")
                .ok_or_else(|| ConfigError::MissingField("schedule cron".to_string()))?;
            Ok(Trigger::Schedule { cron })
        }
        "manual" | "" => Ok(Trigger::Manual),
        _ => Err(ConfigError::InvalidValue {
            field: "trigger type".to_string(),
            message: format!("unknown trigger type: {}", trigger_type),
        }),
    }
}

fn parse_stage(node: &KdlNode) -> ConfigResult<Stage> {
    let name = get_first_string_arg(node)
        .ok_or_else(|| ConfigError::MissingField("stage name".to_string()))?;

    let needs = get_string_list_prop(node, "needs");
    let manual = get_bool_prop(node, "manual").unwrap_or(false);
    let when_expr = get_string_prop(node, "when");

    let when = when_expr.map(|expr| StageCondition { expression: expr });

    let mut image = String::new();
    let mut commands = Vec::new();
    let mut artifacts = Vec::new();
    let mut env = HashMap::new();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "image" => {
                    image = get_first_string_arg(child).unwrap_or_default();
                }
                "run" => {
                    if let Some(cmd) = get_first_string_arg(child) {
                        commands.push(cmd);
                    }
                }
                "artifacts" => {
                    if let Some(art) = get_first_string_arg(child) {
                        artifacts.push(art);
                    }
                }
                "env" => {
                    if let Some(grandchildren) = child.children() {
                        for gc in grandchildren.nodes() {
                            let key = gc.name().value().to_string();
                            if let Some(val) = get_first_string_arg(gc) {
                                env.insert(key, val);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if image.is_empty() {
        return Err(ConfigError::MissingField(format!(
            "image for stage '{}'",
            name
        )));
    }

    Ok(Stage {
        name,
        needs,
        when,
        manual,
        action: StageAction::Run {
            image,
            commands,
            artifacts,
        },
        env,
    })
}

fn parse_cache(node: &KdlNode) -> ConfigResult<CacheConfig> {
    let name = get_first_string_arg(node)
        .ok_or_else(|| ConfigError::MissingField("cache name".to_string()))?;

    let mut paths = Vec::new();
    let mut key = String::new();
    let mut restore_keys = Vec::new();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "path" => {
                    if let Some(p) = get_first_string_arg(child) {
                        paths.push(p);
                    }
                }
                "key" => {
                    key = get_first_string_arg(child).unwrap_or_default();
                }
                "restore_keys" | "restore-keys" => {
                    restore_keys = get_all_string_args(child);
                }
                _ => {}
            }
        }
    }

    Ok(CacheConfig {
        name,
        paths,
        key,
        restore_keys,
    })
}

// Helper functions for extracting values from KDL nodes

fn get_first_string_arg(node: &KdlNode) -> Option<String> {
    node.entries()
        .iter()
        .find(|e| e.name().is_none())
        .and_then(|e| e.value().as_string())
        .map(|s| s.to_string())
}

fn get_all_string_args(node: &KdlNode) -> Vec<String> {
    node.entries()
        .iter()
        .filter(|e| e.name().is_none())
        .filter_map(|e| e.value().as_string())
        .map(|s| s.to_string())
        .collect()
}

fn get_string_prop(node: &KdlNode, name: &str) -> Option<String> {
    node.get(name)
        .and_then(|v| v.as_string())
        .map(|s| s.to_string())
}

fn get_bool_prop(node: &KdlNode, name: &str) -> Option<bool> {
    node.get(name).and_then(|v| v.as_bool())
}

fn get_string_list_prop(node: &KdlNode, name: &str) -> Vec<String> {
    let mut result = Vec::new();

    // First, collect all entries with this name (handles repeated attributes like needs="a" needs="b")
    for entry in node.entries() {
        if let Some(entry_name) = entry.name() {
            if entry_name.value() == name {
                if let Some(s) = entry.value().as_string() {
                    result.push(s.to_string());
                }
            }
        }
    }

    // If we found entries, return them
    if !result.is_empty() {
        return result;
    }

    // Check children for the property name (handles block syntax)
    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == name {
                return get_all_string_args(child);
            }
        }
    }

    Vec::new()
}

/// Detect cycles in the stage dependency graph using DFS.
fn detect_cycle(stages: &[Stage]) -> Result<(), String> {
    let mut visited = HashMap::new();
    let mut rec_stack = HashMap::new();

    let stage_map: HashMap<&str, &Stage> = stages.iter().map(|s| (s.name.as_str(), s)).collect();

    for stage in stages {
        if !visited.contains_key(stage.name.as_str()) {
            if let Some(cycle) =
                dfs_detect_cycle(&stage.name, &stage_map, &mut visited, &mut rec_stack)
            {
                return Err(cycle);
            }
        }
    }
    Ok(())
}

fn dfs_detect_cycle<'a>(
    node: &'a str,
    stage_map: &'a HashMap<&'a str, &'a Stage>,
    visited: &mut HashMap<&'a str, bool>,
    rec_stack: &mut HashMap<&'a str, bool>,
) -> Option<String> {
    visited.insert(node, true);
    rec_stack.insert(node, true);

    if let Some(stage) = stage_map.get(node) {
        for dep in &stage.needs {
            let dep_str: &'a str = dep.as_str();
            if !visited.contains_key(dep_str) {
                if let Some(cycle) = dfs_detect_cycle(dep_str, stage_map, visited, rec_stack) {
                    return Some(cycle);
                }
            } else if rec_stack.get(dep_str).copied().unwrap_or(false) {
                return Some(format!("{} -> {}", node, dep));
            }
        }
    }

    rec_stack.insert(node, false);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pipeline() {
        let kdl = r#"
            pipeline "test-pipeline"

            stage "build" {
                image "rust:1.75"
                run "cargo build"
            }
        "#;

        let pipeline = parse_pipeline(kdl).unwrap();
        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].name, "build");
    }

    #[test]
    fn test_parse_pipeline_with_dependencies() {
        let kdl = r#"
            pipeline "multi-stage"

            stage "test" {
                image "rust:1.75"
                run "cargo test"
            }

            stage "build" needs="test" {
                image "rust:1.75"
                run "cargo build --release"
            }
        "#;

        let pipeline = parse_pipeline(kdl).unwrap();
        assert_eq!(pipeline.stages.len(), 2);
        assert_eq!(pipeline.stages[1].needs, vec!["test"]);
    }

    #[test]
    fn test_detect_missing_dependency() {
        let kdl = r#"
            pipeline "bad-deps"

            stage "build" needs="nonexistent" {
                image "rust:1.75"
                run "cargo build"
            }
        "#;

        let result = parse_pipeline(kdl);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidReference(_)
        ));
    }

    #[test]
    fn test_detect_cycle() {
        let kdl = r#"
            pipeline "cyclic"

            stage "a" needs="b" {
                image "alpine"
                run "echo a"
            }

            stage "b" needs="a" {
                image "alpine"
                run "echo b"
            }
        "#;

        let result = parse_pipeline(kdl);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::CycleDetected(_)));
    }
}
