use std::collections::BTreeMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

use serde::Serialize;

use crate::cli::truncate_id;
use crate::containers::error::{DockerError, Result};
use crate::session::repo_config::HookProgress;
use crate::session::ComposeConfig;

use super::container_interface::ContainerConfig;

/// Docker Compose engine for managing agent containers via compose overlays.
///
/// Instead of `docker run`, this generates a compose overlay YAML that defines
/// the agent service and uses `docker compose up/down/exec` for lifecycle management.
pub struct ComposeEngine {
    pub project_name: String,
    pub compose_files: Vec<PathBuf>,
    pub overlay_path: PathBuf,
    pub agent_service: String,
}

impl ComposeEngine {
    /// Create a new ComposeEngine from session and config state.
    ///
    /// - `session_id`: used to derive the project name
    /// - `project_path`: base path for resolving relative compose file paths
    /// - `compose_config`: the `[sandbox.compose]` config section
    /// - `app_dir`: AoE app directory for storing overlay files
    pub fn new(
        session_id: &str,
        project_path: &Path,
        compose_config: &ComposeConfig,
        app_dir: &Path,
    ) -> Self {
        let project_name = format!("aoe-{}", truncate_id(session_id, 8));
        let compose_files = compose_config
            .compose_files
            .iter()
            .map(|f| project_path.join(f))
            .collect();
        let overlay_path = app_dir
            .join("compose-overlays")
            .join(format!("{}.override.yaml", project_name));

        Self {
            project_name,
            compose_files,
            overlay_path,
            agent_service: compose_config.agent_service.clone(),
        }
    }

    /// Build the base `docker compose` argument list shared by all commands.
    ///
    /// Produces: `["compose", "-f", user1, "-f", user2, ..., "-f", overlay, "-p", project_name]`
    pub fn compose_base_args(&self) -> Vec<String> {
        let mut args = vec!["compose".to_string()];
        for f in &self.compose_files {
            args.push("-f".to_string());
            args.push(f.display().to_string());
        }
        args.push("-f".to_string());
        args.push(self.overlay_path.display().to_string());
        args.push("-p".to_string());
        args.push(self.project_name.clone());
        args
    }

    /// Build the base args as a single string (for exec_command shell interpolation).
    fn compose_base_args_str(&self) -> String {
        let mut parts = vec!["docker".to_string(), "compose".to_string()];
        for f in &self.compose_files {
            parts.push("-f".to_string());
            parts.push(shell_quote(&f.display().to_string()));
        }
        parts.push("-f".to_string());
        parts.push(shell_quote(&self.overlay_path.display().to_string()));
        parts.push("-p".to_string());
        parts.push(self.project_name.clone());
        parts.join(" ")
    }

    /// Generate the overlay YAML file from a ContainerConfig.
    pub fn generate_overlay(&self, config: &ContainerConfig, image: &str) -> Result<()> {
        let overlay_dir = self
            .overlay_path
            .parent()
            .ok_or_else(|| DockerError::ComposeOverlayFailed("Invalid overlay path".to_string()))?;
        fs::create_dir_all(overlay_dir)?;

        let yaml = build_overlay_yaml(&self.agent_service, config, image);

        // Write atomically via temp file + rename
        let tmp_path = self.overlay_path.with_extension("yaml.tmp");
        fs::write(&tmp_path, &yaml).map_err(|e| {
            DockerError::ComposeOverlayFailed(format!(
                "Failed to write overlay to {}: {}",
                tmp_path.display(),
                e
            ))
        })?;
        fs::rename(&tmp_path, &self.overlay_path).map_err(|e| {
            DockerError::ComposeOverlayFailed(format!(
                "Failed to rename overlay to {}: {}",
                self.overlay_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Delete the overlay file from disk.
    pub fn cleanup_overlay(&self) -> Result<()> {
        if self.overlay_path.exists() {
            fs::remove_file(&self.overlay_path)?;
        }
        Ok(())
    }

    /// Check that `docker compose` v2 is available.
    pub fn check_compose_available() -> Result<()> {
        let output = Command::new("docker")
            .args(["compose", "version"])
            .output()
            .map_err(|_| DockerError::ComposeNotInstalled)?;

        if !output.status.success() {
            return Err(DockerError::ComposeNotInstalled);
        }
        Ok(())
    }

    /// Start the compose stack: `docker compose ... up -d`
    pub fn up(&self, progress: Option<&mpsc::Sender<HookProgress>>) -> Result<()> {
        let mut args = self.compose_base_args();
        args.extend(["up".to_string(), "-d".to_string()]);

        if let Some(tx) = progress {
            let _ = tx.send(HookProgress::Started("docker compose up".to_string()));
        }

        run_compose_streamed(&args, progress)
    }

    /// Stop and remove the compose stack: `docker compose ... down [--volumes]`
    pub fn down(
        &self,
        remove_volumes: bool,
        progress: Option<&mpsc::Sender<HookProgress>>,
    ) -> Result<()> {
        let mut args = self.compose_base_args();
        args.push("down".to_string());
        if remove_volumes {
            args.push("--volumes".to_string());
        }

        if let Some(tx) = progress {
            let _ = tx.send(HookProgress::Started("docker compose down".to_string()));
        }

        let result = run_compose_streamed(&args, progress);
        if let Err(ref e) = result {
            tracing::warn!("docker compose down failed: {}", e);
        }
        // down() historically swallows errors -- preserve that behavior
        Ok(())
    }

    /// Check if the agent service is running.
    pub fn is_running(&self) -> Result<bool> {
        let mut args = self.compose_base_args();
        args.extend([
            "ps".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "--status".to_string(),
            "running".to_string(),
            self.agent_service.clone(),
        ]);

        let output = Command::new("docker").args(&args).output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_compose_ps_has_service(&stdout, &self.agent_service))
    }

    /// Check if the agent service exists in any state.
    pub fn exists(&self) -> Result<bool> {
        let mut args = self.compose_base_args();
        args.extend([
            "ps".to_string(),
            "--format".to_string(),
            "json".to_string(),
            self.agent_service.clone(),
        ]);

        let output = Command::new("docker").args(&args).output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_compose_ps_has_service(&stdout, &self.agent_service))
    }

    /// Build an interactive exec command string (for tmux/shell embedding).
    ///
    /// Format: `docker compose -f ... -p ... exec [options] <service>`
    pub fn exec_command(&self, options: Option<&str>) -> String {
        let base = self.compose_base_args_str();
        if let Some(opts) = options {
            format!("{} exec {} {}", base, opts, self.agent_service)
        } else {
            format!("{} exec {}", base, self.agent_service)
        }
    }

    /// Run a non-interactive exec command and return the output.
    pub fn exec(&self, cmd: &[&str]) -> Result<std::process::Output> {
        let mut args = self.compose_base_args();
        args.push("exec".to_string());
        args.push("-T".to_string());
        args.push(self.agent_service.clone());
        args.extend(cmd.iter().map(|s| s.to_string()));

        let output = Command::new("docker").args(&args).output()?;
        Ok(output)
    }
}

/// Run a docker compose command, optionally streaming stderr to a progress channel.
///
/// When `progress` is `Some`, stderr is piped and sent line-by-line. When `None`,
/// the command runs with captured output (current behavior).
/// Returns the collected stderr on failure for error reporting.
fn run_compose_streamed(
    args: &[String],
    progress: Option<&mpsc::Sender<HookProgress>>,
) -> Result<()> {
    match progress {
        Some(tx) => {
            let mut child = Command::new("docker")
                .args(args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()?;

            let mut collected_stderr = String::new();
            if let Some(stderr) = child.stderr.take() {
                let reader = std::io::BufReader::new(stderr);
                for line in reader.lines().map_while(|r| r.ok()) {
                    collected_stderr.push_str(&line);
                    collected_stderr.push('\n');
                    let _ = tx.send(HookProgress::Output(line));
                }
            }

            let status = child.wait()?;
            if !status.success() {
                return Err(DockerError::ComposeCommandFailed(
                    collected_stderr.trim().to_string(),
                ));
            }
            Ok(())
        }
        None => {
            let output = Command::new("docker").args(args).output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(DockerError::ComposeCommandFailed(stderr.trim().to_string()));
            }
            Ok(())
        }
    }
}

/// Parse NDJSON output from `docker compose ps --format json` to check if a service exists.
fn parse_compose_ps_has_service(output: &str, service_name: &str) -> bool {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if val.get("Service").and_then(|v| v.as_str()) == Some(service_name) {
                return true;
            }
        }
    }
    false
}

/// Top-level compose overlay structure for serde_yaml serialization.
#[derive(Serialize)]
struct ComposeOverlay {
    services: BTreeMap<String, ServiceDef>,
}

#[derive(Serialize)]
struct ServiceDef {
    image: String,
    command: String,
    stdin_open: bool,
    tty: bool,
    working_dir: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    volumes: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    environment: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deploy: Option<DeployDef>,
}

#[derive(Serialize)]
struct DeployDef {
    resources: ResourcesDef,
}

#[derive(Serialize)]
struct ResourcesDef {
    limits: LimitsDef,
}

#[derive(Serialize)]
struct LimitsDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    cpus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<String>,
}

/// Build the overlay YAML string from a ContainerConfig using serde_yaml.
fn build_overlay_yaml(service_name: &str, config: &ContainerConfig, image: &str) -> String {
    let mut volumes = Vec::new();
    for vol in &config.volumes {
        if vol.read_only {
            volumes.push(format!("{}:{}:ro", vol.host_path, vol.container_path));
        } else {
            volumes.push(format!("{}:{}", vol.host_path, vol.container_path));
        }
    }
    for anon in &config.anonymous_volumes {
        volumes.push(anon.clone());
    }

    let environment: BTreeMap<String, String> = config.environment.iter().cloned().collect();

    let deploy = if config.cpu_limit.is_some() || config.memory_limit.is_some() {
        Some(DeployDef {
            resources: ResourcesDef {
                limits: LimitsDef {
                    cpus: config.cpu_limit.clone(),
                    memory: config.memory_limit.clone(),
                },
            },
        })
    } else {
        None
    };

    let service = ServiceDef {
        image: image.to_string(),
        command: "sleep infinity".to_string(),
        stdin_open: true,
        tty: true,
        working_dir: config.working_dir.clone(),
        volumes,
        environment,
        deploy,
    };

    let mut services = BTreeMap::new();
    services.insert(service_name.to_string(), service);

    let overlay = ComposeOverlay { services };

    serde_yaml::to_string(&overlay).expect("ComposeOverlay serialization should never fail")
}

/// Shell-quote a string for safe inclusion in command lines.
///
/// Keeps simple alphanumeric/path strings unquoted for readability, but
/// uses single-quoting for everything else as it's the most robust
/// cross-shell quoting mechanism.
fn shell_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '.' || c == '_' || c == '-')
    {
        return s.to_string();
    }
    // Single-quote everything else, escaping existing single quotes
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::containers::container_interface::VolumeMount;

    #[test]
    fn test_compose_engine_new() {
        let config = ComposeConfig {
            compose_files: vec!["docker-compose.yml".to_string()],
            agent_service: "aoe-agent".to_string(),
        };
        let engine = ComposeEngine::new(
            "abcdefghijklmnop",
            Path::new("/home/user/project"),
            &config,
            Path::new("/home/user/.config/agent-of-empires"),
        );

        assert_eq!(engine.project_name, "aoe-abcdefgh");
        assert_eq!(
            engine.compose_files,
            vec![PathBuf::from("/home/user/project/docker-compose.yml")]
        );
        assert_eq!(
            engine.overlay_path,
            PathBuf::from(
                "/home/user/.config/agent-of-empires/compose-overlays/aoe-abcdefgh.override.yaml"
            )
        );
        assert_eq!(engine.agent_service, "aoe-agent");
    }

    #[test]
    fn test_compose_base_args() {
        let engine = ComposeEngine {
            project_name: "aoe-abc12345".to_string(),
            compose_files: vec![
                PathBuf::from("/project/docker-compose.yml"),
                PathBuf::from("/project/docker-compose.db.yml"),
            ],
            overlay_path: PathBuf::from("/app/compose-overlays/aoe-abc12345.override.yaml"),
            agent_service: "aoe-agent".to_string(),
        };

        let args = engine.compose_base_args();
        assert_eq!(
            args,
            vec![
                "compose",
                "-f",
                "/project/docker-compose.yml",
                "-f",
                "/project/docker-compose.db.yml",
                "-f",
                "/app/compose-overlays/aoe-abc12345.override.yaml",
                "-p",
                "aoe-abc12345",
            ]
        );
    }

    #[test]
    fn test_build_overlay_yaml_basic() {
        let config = ContainerConfig {
            working_dir: "/workspace/myproject".to_string(),
            volumes: vec![VolumeMount {
                host_path: "/home/user/project".to_string(),
                container_path: "/workspace/myproject".to_string(),
                read_only: false,
            }],
            anonymous_volumes: vec![],
            environment: vec![("TERM".to_string(), "xterm-256color".to_string())],
            cpu_limit: None,
            memory_limit: None,
        };

        let yaml = build_overlay_yaml("aoe-agent", &config, "ghcr.io/njbrake/aoe-sandbox:latest");

        // Verify the YAML parses correctly
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let service = &parsed["services"]["aoe-agent"];
        assert_eq!(
            service["image"].as_str().unwrap(),
            "ghcr.io/njbrake/aoe-sandbox:latest"
        );
        assert_eq!(service["command"].as_str().unwrap(), "sleep infinity");
        assert_eq!(service["stdin_open"].as_bool().unwrap(), true);
        assert_eq!(service["tty"].as_bool().unwrap(), true);
        assert_eq!(
            service["working_dir"].as_str().unwrap(),
            "/workspace/myproject"
        );
        assert_eq!(
            service["volumes"][0].as_str().unwrap(),
            "/home/user/project:/workspace/myproject"
        );
        assert_eq!(
            service["environment"]["TERM"].as_str().unwrap(),
            "xterm-256color"
        );
        // No deploy section when no limits
        assert!(service["deploy"].is_null());
    }

    #[test]
    fn test_build_overlay_yaml_with_readonly_volumes() {
        let config = ContainerConfig {
            working_dir: "/workspace".to_string(),
            volumes: vec![
                VolumeMount {
                    host_path: "/home/user/.gitconfig".to_string(),
                    container_path: "/root/.gitconfig".to_string(),
                    read_only: true,
                },
                VolumeMount {
                    host_path: "/home/user/.ssh".to_string(),
                    container_path: "/root/.ssh".to_string(),
                    read_only: true,
                },
            ],
            anonymous_volumes: vec![],
            environment: vec![],
            cpu_limit: None,
            memory_limit: None,
        };

        let yaml = build_overlay_yaml("aoe-agent", &config, "ubuntu:latest");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let vols = &parsed["services"]["aoe-agent"]["volumes"];

        assert_eq!(
            vols[0].as_str().unwrap(),
            "/home/user/.gitconfig:/root/.gitconfig:ro"
        );
        assert_eq!(vols[1].as_str().unwrap(), "/home/user/.ssh:/root/.ssh:ro");
    }

    #[test]
    fn test_build_overlay_yaml_with_anonymous_volumes() {
        let config = ContainerConfig {
            working_dir: "/workspace/myproject".to_string(),
            volumes: vec![],
            anonymous_volumes: vec![
                "/workspace/myproject/node_modules".to_string(),
                "/workspace/myproject/target".to_string(),
            ],
            environment: vec![],
            cpu_limit: None,
            memory_limit: None,
        };

        let yaml = build_overlay_yaml("aoe-agent", &config, "ubuntu:latest");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let vols = &parsed["services"]["aoe-agent"]["volumes"];

        assert_eq!(
            vols[0].as_str().unwrap(),
            "/workspace/myproject/node_modules"
        );
        assert_eq!(vols[1].as_str().unwrap(), "/workspace/myproject/target");
    }

    #[test]
    fn test_build_overlay_yaml_with_resource_limits() {
        let config = ContainerConfig {
            working_dir: "/workspace".to_string(),
            volumes: vec![],
            anonymous_volumes: vec![],
            environment: vec![],
            cpu_limit: Some("2".to_string()),
            memory_limit: Some("4g".to_string()),
        };

        let yaml = build_overlay_yaml("aoe-agent", &config, "ubuntu:latest");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let limits = &parsed["services"]["aoe-agent"]["deploy"]["resources"]["limits"];

        assert_eq!(limits["cpus"].as_str().unwrap(), "2");
        assert_eq!(limits["memory"].as_str().unwrap(), "4g");
    }

    #[test]
    fn test_build_overlay_yaml_cpu_only() {
        let config = ContainerConfig {
            working_dir: "/workspace".to_string(),
            volumes: vec![],
            anonymous_volumes: vec![],
            environment: vec![],
            cpu_limit: Some("0.5".to_string()),
            memory_limit: None,
        };

        let yaml = build_overlay_yaml("aoe-agent", &config, "ubuntu:latest");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let limits = &parsed["services"]["aoe-agent"]["deploy"]["resources"]["limits"];

        assert_eq!(limits["cpus"].as_str().unwrap(), "0.5");
        assert!(limits["memory"].is_null());
    }

    #[test]
    fn test_build_overlay_yaml_env_special_chars() {
        let config = ContainerConfig {
            working_dir: "/workspace".to_string(),
            volumes: vec![],
            anonymous_volumes: vec![],
            environment: vec![
                ("PATH".to_string(), "/usr/local/bin:/usr/bin".to_string()),
                ("MSG".to_string(), "hello \"world\"".to_string()),
                ("NEWLINE".to_string(), "line1\nline2".to_string()),
            ],
            cpu_limit: None,
            memory_limit: None,
        };

        let yaml = build_overlay_yaml("aoe-agent", &config, "ubuntu:latest");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let env = &parsed["services"]["aoe-agent"]["environment"];

        assert_eq!(env["PATH"].as_str().unwrap(), "/usr/local/bin:/usr/bin");
        assert_eq!(env["MSG"].as_str().unwrap(), "hello \"world\"");
        assert_eq!(env["NEWLINE"].as_str().unwrap(), "line1\nline2");
    }

    #[test]
    fn test_build_overlay_yaml_custom_service_name() {
        let config = ContainerConfig {
            working_dir: "/workspace".to_string(),
            volumes: vec![],
            anonymous_volumes: vec![],
            environment: vec![],
            cpu_limit: None,
            memory_limit: None,
        };

        let yaml = build_overlay_yaml("my-custom-agent", &config, "ubuntu:latest");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();

        assert!(parsed["services"]["my-custom-agent"].is_mapping());
    }

    #[test]
    fn test_build_overlay_yaml_env_sorted() {
        let config = ContainerConfig {
            working_dir: "/workspace".to_string(),
            volumes: vec![],
            anonymous_volumes: vec![],
            environment: vec![
                ("ZEBRA".to_string(), "z".to_string()),
                ("ALPHA".to_string(), "a".to_string()),
                ("MIDDLE".to_string(), "m".to_string()),
            ],
            cpu_limit: None,
            memory_limit: None,
        };

        let yaml = build_overlay_yaml("agent", &config, "ubuntu:latest");

        // BTreeMap sorts keys, so ALPHA should come before MIDDLE before ZEBRA
        let alpha_pos = yaml.find("ALPHA").unwrap();
        let middle_pos = yaml.find("MIDDLE").unwrap();
        let zebra_pos = yaml.find("ZEBRA").unwrap();
        assert!(alpha_pos < middle_pos);
        assert!(middle_pos < zebra_pos);
    }

    #[test]
    fn test_exec_command_no_options() {
        let engine = ComposeEngine {
            project_name: "aoe-abc12345".to_string(),
            compose_files: vec![PathBuf::from("/project/compose.yml")],
            overlay_path: PathBuf::from("/app/overlays/aoe-abc12345.override.yaml"),
            agent_service: "aoe-agent".to_string(),
        };

        let cmd = engine.exec_command(None);
        assert!(cmd.starts_with("docker compose"));
        assert!(cmd.contains("-f /project/compose.yml"));
        assert!(cmd.contains("-f /app/overlays/aoe-abc12345.override.yaml"));
        assert!(cmd.contains("-p aoe-abc12345"));
        assert!(cmd.ends_with("exec aoe-agent"));
    }

    #[test]
    fn test_exec_command_with_quoted_paths() {
        let engine = ComposeEngine {
            project_name: "aoe-abc12345".to_string(),
            compose_files: vec![PathBuf::from("/project folder/compose.yml")],
            overlay_path: PathBuf::from("/app/overlays/aoe-abc12345.override.yaml"),
            agent_service: "aoe-agent".to_string(),
        };

        let cmd = engine.exec_command(None);
        assert!(cmd.contains("-f '/project folder/compose.yml'"));
    }

    #[test]
    fn test_exec_command_with_options() {
        let engine = ComposeEngine {
            project_name: "aoe-abc12345".to_string(),
            compose_files: vec![PathBuf::from("/project/compose.yml")],
            overlay_path: PathBuf::from("/app/overlays/aoe-abc12345.override.yaml"),
            agent_service: "aoe-agent".to_string(),
        };

        let cmd = engine.exec_command(Some("-w /workspace -e FOO=bar"));
        assert!(cmd.contains("exec -w /workspace -e FOO=bar aoe-agent"));
    }

    #[test]
    fn test_parse_compose_ps_has_service_found() {
        let output = r#"{"ID":"abc123","Name":"proj-aoe-agent-1","Service":"aoe-agent","State":"running"}
{"ID":"def456","Name":"proj-db-1","Service":"db","State":"running"}"#;

        assert!(parse_compose_ps_has_service(output, "aoe-agent"));
        assert!(parse_compose_ps_has_service(output, "db"));
        assert!(!parse_compose_ps_has_service(output, "redis"));
    }

    #[test]
    fn test_parse_compose_ps_has_service_empty() {
        assert!(!parse_compose_ps_has_service("", "aoe-agent"));
        assert!(!parse_compose_ps_has_service("  \n  \n", "aoe-agent"));
    }

    #[test]
    fn test_shell_quote() {
        assert_eq!(shell_quote("simple"), "simple");
        assert_eq!(shell_quote("/path/to/file.yml"), "/path/to/file.yml");
        assert_eq!(shell_quote("with space"), "'with space'");
        assert_eq!(shell_quote("it's-a-me"), "'it'\\''s-a-me'");
        assert_eq!(shell_quote(""), "''");
    }
}
