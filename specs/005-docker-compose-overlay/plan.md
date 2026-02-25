# Implementation Plan: Docker Compose Overlay

## Checklist

- [x] Step 1: Config types and validation
- [x] Step 2: ComposeEngine struct and overlay generation
- [x] Step 3: ComposeEngine lifecycle methods
- [x] Step 4: Instance integration (session start)
- [x] Step 5: Exec integration (agent launch and terminal)
- [x] Step 6: Cleanup integration (session deletion)
- [x] Step 7: TUI settings and profile overrides
- [x] Step 8: Status poller and health checks

---

## Step 1: Config types and validation

**Objective**: Add the `Compose` runtime variant and `ComposeConfig` struct so AoE can be configured for compose mode.

**Implementation guidance**:

Files to modify:
- `src/session/config.rs`: Add `Compose` variant to `ContainerRuntimeName`. Add `ComposeConfig` struct. Add `compose: Option<ComposeConfig>` field to `SandboxConfig` with `#[serde(default)]`.
- `src/session/profile_config.rs`: Add `ComposeConfigOverride` struct and add `compose` field to `SandboxConfigOverride`. Wire up merge logic in `merge_configs()`.

`ComposeConfig`:
```rust
pub struct ComposeConfig {
    pub compose_files: Vec<String>,
    #[serde(default = "default_agent_service")]
    pub agent_service: String,
}
```

Add a validation function `validate_compose_config(sandbox: &SandboxConfig) -> Result<()>` that checks:
- If runtime = Compose, `compose` must be `Some`
- `compose_files` must not be empty
- Runtime must not be `AppleContainer` (Compose is Docker-only)

**Test requirements**:
- Unit test: serialize/deserialize `SandboxConfig` with compose section roundtrips correctly
- Unit test: `validate_compose_config` errors on missing compose section, empty files list
- Unit test: default `agent_service` is `"aoe-agent"` when not specified
- Unit test: profile override merge for compose fields

**Integration notes**: No behavioral changes yet. Existing Docker/AppleContainer paths are unaffected. Users can set the config but nothing will happen until Step 4.

**Demo**: `cargo test` passes. Config file with `[sandbox.compose]` section parses correctly.

---

## Step 2: ComposeEngine struct and overlay generation

**Objective**: Create the `ComposeEngine` struct and implement overlay YAML generation from `ContainerConfig`.

**Implementation guidance**:

New file: `src/containers/compose.rs`
- Expose module in `src/containers/mod.rs`

`ComposeEngine` struct:
```rust
pub struct ComposeEngine {
    project_name: String,
    compose_files: Vec<PathBuf>,
    overlay_path: PathBuf,
    agent_service: String,
}
```

Constructor `ComposeEngine::new(session_id, project_path, compose_config, app_dir)`:
- Derive `project_name`: `aoe-{truncate_id(session_id, 8)}`
- Resolve `compose_files`: each config path joined with `project_path`
- Derive `overlay_path`: `{app_dir}/compose-overlays/{project_name}.override.yaml`

`generate_overlay(&self, config: &ContainerConfig, image: &str) -> Result<()>`:
- Create `compose-overlays/` directory if needed
- Build YAML string via templating:
  - Service preamble with `image`, `command: sleep infinity`, `stdin_open: true`, `tty: true`, `working_dir`
  - `volumes:` block from `config.volumes` (append `:ro` for read_only) and `config.anonymous_volumes`
  - `environment:` block from `config.environment` (YAML-escape values containing `:`, `#`, quotes, newlines)
  - `deploy.resources.limits` block if `cpu_limit` or `memory_limit` is set
- Write to `overlay_path` atomically (write to `.tmp` then rename, to avoid partial reads)

`cleanup_overlay(&self) -> Result<()>`:
- Delete `overlay_path` if it exists

Helper `compose_base_args(&self) -> Vec<String>`:
- Returns `["compose", "-f", file1, "-f", file2, ..., "-f", overlay, "-p", project_name]`

YAML escaping helper:
- Double-quote all environment values
- Escape `\` and `"` within values
- Quote volume paths that contain spaces

**Test requirements**:
- Unit test: `generate_overlay` with a known `ContainerConfig` produces correct YAML containing:
  - Correct service name, image, command
  - All bind mount volumes with correct `:ro` suffix
  - All anonymous volumes (path-only, no host)
  - All environment variables with proper quoting
  - Resource limits under `deploy.resources.limits` with correct format
- Unit test: no `deploy` section when both limits are `None`
- Unit test: YAML escaping for values with colons, quotes, hash characters
- Unit test: `compose_base_args` produces correct `-f` flag ordering (user files first, overlay last)
- Unit test: overlay path and project name derivation

**Integration notes**: No runtime behavior yet. The engine can generate files but doesn't call `docker compose`.

**Demo**: Unit tests produce and verify overlay YAML content.

---

## Step 3: ComposeEngine lifecycle methods

**Objective**: Implement `up`, `down`, `is_running`, `exists`, and `check_compose_available`.

**Implementation guidance**:

File: `src/containers/compose.rs`

`check_compose_available() -> Result<()>`:
- Run `docker compose version`, check exit code
- Parse version string for informational logging
- Return clear error if not available

`up(&self) -> Result<()>`:
- Run `docker compose {base_args} up -d`
- Capture stderr on failure, return as error context

`down(&self, remove_volumes: bool) -> Result<()>`:
- Run `docker compose {base_args} down [--volumes]`
- Best-effort: log warning on failure, don't propagate hard errors (cleanup path)

`is_running(&self) -> Result<bool>`:
- Run `docker compose {base_args} ps --format json --status running {agent_service}`
- Parse NDJSON output line-by-line
- Return `true` if any line has `Service == agent_service` and `State == "running"`
- Empty output = not running

`exists(&self) -> Result<bool>`:
- Run `docker compose {base_args} ps --format json {agent_service}`
- Return `true` if any output exists for the service (any state)

**Test requirements**:
- Integration test (requires Docker + Compose): Create a minimal compose file (empty `services:` section), generate overlay with agent service, run `up`, verify `is_running()` returns true, run `down`, verify `is_running()` returns false.
- Unit test: NDJSON parsing logic (mock output strings)
- Unit test: `check_compose_available` with mock command output

**Integration notes**: These methods are standalone -- no changes to existing code paths yet. Can be tested independently.

**Demo**: Integration test starts and stops a real compose stack.

---

## Step 4: Instance integration (session start)

**Objective**: Wire `ComposeEngine` into `ensure_container_running` so Compose sessions actually start.

**Implementation guidance**:

Files to modify:
- `src/session/instance.rs`: Modify `get_container_for_instance()` (or create a parallel method for compose)
- `src/session/builder.rs`: Add compose availability check in `build_instance()`
- `src/containers/mod.rs`: Update `get_container_runtime()` to handle `Compose` variant (for availability checks only -- actual lifecycle uses `ComposeEngine` directly)
- `src/cli/add.rs`: Update runtime availability check for the `add` command

Approach in `instance.rs`:
- At the top of `get_container_for_instance()`, check `container_runtime`
- If `Compose`: build `ComposeEngine` from session + config, call `validate_compose_config`, call `check_compose_available`, then:
  - If `engine.is_running()` -> return (already running)
  - If `!engine.exists()` -> call `build_container_config()`, `engine.generate_overlay()`, `engine.up()`
  - If exists but not running -> `engine.up()` (restart)
- Store or return `ComposeEngine` for later use by exec paths (likely store runtime mode info on Instance or return alongside container)

In `builder.rs`:
- When `container_runtime = Compose`, call `ComposeEngine::check_compose_available()` instead of `runtime.is_daemon_running()` (though Docker daemon still needs to be running -- check both)

The `SandboxInfo.container_name` field: for Compose mode, store the project name (e.g., `aoe-abc12345`) instead of a container name. This is used for display/identification. The `container_id` can remain `None` since compose manages containers.

**Test requirements**:
- Integration test: Create a session with compose runtime, verify overlay is generated and stack is running
- Integration test: Restart AoE (simulate by reconstructing engine), verify `is_running()` returns true without regenerating overlay
- Unit test: compose config validation is called during session start

**Integration notes**: This is the first step where compose mode is end-to-end functional for starting a session. Agent exec is not wired yet (Step 5), so the session starts but agents can't be launched.

**Demo**: Start a session with `container_runtime = Compose`, see the compose stack come up via `docker compose ps`.

---

## Step 5: Exec integration (agent launch and terminal)

**Objective**: Wire `ComposeEngine::exec_command` into agent launch and container terminal paths.

**Implementation guidance**:

File: `src/containers/compose.rs`

`exec_command(&self, options: Option<&str>) -> String`:
- Build: `docker compose {base_args_as_string} exec {options} {agent_service}`
- Note: no `-it` flags needed (compose exec has TTY on by default)
- The `options` parameter carries `-w` and `-e` flags, same format as Docker mode

`exec(&self, cmd: &[&str]) -> Result<std::process::Output>`:
- Build: `docker compose {base_args} exec -T {agent_service} {cmd...}`
- `-T` for non-interactive (scripted) exec

Files to modify:
- `src/session/instance.rs`:
  - `start_with_size_opts()` (line ~402): where `container.exec_command()` is called for tool launch -- dispatch to `engine.exec_command()` when compose
  - `start_container_terminal_with_size()` (line ~251): where interactive shell exec is built -- dispatch to `engine.exec_command()` when compose
  - `execute_hooks_in_container()`: if this uses container exec, dispatch similarly

The compose exec command string format:
```
docker compose -f /path/user.yaml -f /path/overlay.yaml -p aoe-abc12345 exec -w /workspace/project -e VAR=val aoe-agent <tool_cmd>
```

**Test requirements**:
- Unit test: `exec_command` builds correct command string with and without options
- Unit test: `exec` builds correct command with `-T` flag
- Integration test: exec into running compose agent, run a simple command (`echo hello`), verify output

**Integration notes**: After this step, agents can be launched and container terminals work in compose mode. This is the first fully functional end-to-end demo.

**Demo**: Start a compose session, launch an agent, open a container terminal. All work via `docker compose exec`.

---

## Step 6: Cleanup integration (session deletion)

**Objective**: Wire compose cleanup into all deletion paths.

**Implementation guidance**:

Files to modify:
- `src/cli/remove.rs` (line ~129): Replace `DockerContainer::from_session_id` + `container.remove()` with runtime-aware dispatch. When compose: `ComposeEngine::new(...).down(true)` + `cleanup_overlay()`.
- `src/tui/deletion_poller.rs` (line ~112): Same pattern -- dispatch to compose cleanup when runtime = Compose.
- `src/session/builder.rs` (line ~224): `cleanup_instance()` on build failure -- dispatch to compose cleanup.
- `src/cli/session.rs` (line ~133): `stop_session()` -- dispatch to compose stop.
- `src/session/instance.rs` (line ~554): `stop()` method -- dispatch to `engine.down(false)` (stop without removing volumes, since this is stop not delete).

For the stop vs delete distinction:
- **Stop** (`instance.stop()`): `docker compose down` without `--volumes` (preserves data, just stops containers)
- **Delete** (`remove.rs`, `deletion_poller.rs`): `docker compose down --volumes` + `cleanup_overlay()`

Need to determine runtime at each call site. Options:
- Load config and check `container_runtime`
- Or store runtime type in `SandboxInfo` (add a field) so it's available without loading config

Recommendation: Add `container_runtime: ContainerRuntimeName` to `SandboxInfo`. This is already serialized with the session, so the runtime is known even if config changes. This also helps the deletion path which may not have easy access to the current config.

This is a **breaking change** to the SandboxInfo serialization. Add a migration (`src/migrations/`) that defaults existing sessions to `Docker` if the field is missing (or use `#[serde(default)]`).

**Test requirements**:
- Integration test: Start a compose session, delete it, verify stack is down (`docker compose ps` returns nothing) and overlay file is removed
- Integration test: Stop a compose session (not delete), verify containers stop but overlay persists
- Unit test: cleanup_overlay removes the file

**Integration notes**: After this step, the full session lifecycle works: start, exec, stop, delete. All compose-specific.

**Demo**: Full lifecycle: create session -> run agent -> stop session -> delete session. Verify clean state.

---

## Step 7: TUI settings and profile overrides

**Objective**: Expose Compose configuration in the settings TUI with dynamic visibility.

**Implementation guidance**:

Files to modify:
- `src/tui/settings/fields.rs`:
  - Add `FieldKey::ComposeFiles` and `FieldKey::ComposeAgentService` variants
  - Add `"Compose"` to the `ContainerRuntime` select field options
  - Add two new `SettingField` entries in `build_sandbox_fields()`:
    - `ComposeFiles`: List type, default empty
    - `ComposeAgentService`: Text type, default `"aoe-agent"`
  - Add `apply_field_to_global()` cases for both new keys
  - Add `apply_field_to_profile()` cases for both new keys

- `src/tui/settings/input.rs`:
  - Add `clear_profile_override()` cases for new keys

Dynamic visibility: Add a filtering mechanism so `ComposeFiles` and `ComposeAgentService` are only included in the rendered field list when the current `container_runtime` value is `Compose`. The simplest approach:
- In `build_sandbox_fields()`, tag compose-specific fields with a marker (e.g., a `visible_when` field on `SettingField`, or simply filter them out based on current runtime value)
- When the runtime selection changes, rebuild/refilter the field list

Check if there's existing precedent for conditional field visibility in the TUI. If not, the simplest approach is to conditionally include the fields in `build_sandbox_fields()` based on the current config value, and trigger a field rebuild when the runtime select changes.

**Test requirements**:
- Manual test: Open settings, select Compose runtime, verify ComposeFiles and ComposeAgentService appear
- Manual test: Switch back to Docker, verify compose fields disappear
- Manual test: Add compose files, save, reload -- verify config.toml has `[sandbox.compose]` section
- Unit test: `apply_field_to_global` correctly updates compose config

**Integration notes**: Purely UI. No behavioral changes to runtime.

**Demo**: Full TUI walkthrough: switch to Compose, configure files and service name, save, verify config.

---

## Step 8: Status poller and health checks

**Objective**: Make the TUI session list show correct running/stopped status for compose sessions.

**Implementation guidance**:

Files to modify:
- `src/tui/status_poller.rs`: The `batch_container_health()` function currently calls `runtime.batch_running_states("aoe-sandbox-")` to check all containers at once. For compose sessions, this won't work because compose containers are named by compose (e.g., `aoe-abc12345-aoe-agent-1`), not `aoe-sandbox-*`.

Approach:
- For compose sessions, query each session individually via `ComposeEngine::is_running()`
- Or: use `docker compose ls --format json` to list all compose projects matching `aoe-*` prefix, then check which are running
- Or: since the container naming follows a predictable pattern (`{project_name}-{service_name}-1`), the batch approach can use that prefix

Simplest approach: extend `batch_container_health()` to also check compose containers. The compose container name pattern is `{project_name}-{agent_service}-{replica}` (e.g., `aoe-abc12345-aoe-agent-1`). We can include this pattern in the existing `docker ps` batch query, or add a separate compose-specific batch call.

Alternatively, if compose sessions are few, individual `is_running()` calls per session may be acceptable. Profile this based on expected usage.

- `src/tui/creation_poller.rs`: For compose mode, the creation flow is different (no image pre-pull, just `generate_overlay` + `up`). Ensure the creation poller handles compose sessions correctly or bypasses the image-pull progress UI.

- `src/tui/dialogs/new_session/mod.rs` (line ~282, ~332, ~385): Runtime availability checks and image existence checks. For compose mode:
  - Availability: check both Docker daemon and `docker compose version`
  - Image: skip the local image check (compose handles pulling)
  - Default image: still use `default_image` from config

**Test requirements**:
- Manual test: Start a compose session, verify TUI shows "Running" status
- Manual test: Stop compose session externally (`docker compose down`), verify TUI updates to stopped/missing
- Manual test: Create new session dialog works with Compose runtime selected

**Integration notes**: This is the polish step. After this, compose mode is fully integrated into the TUI experience.

**Demo**: Full TUI experience: create compose session from new-session dialog, see running status, agents work, delete session cleanly.
