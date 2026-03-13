# Docker Compose Overlay -- Summary

## Artifacts

| File | Description |
|---|---|
| `specs/docker-compose-overlay/rough-idea.md` | Original concept |
| `specs/docker-compose-overlay/requirements.md` | 18 Q&A clarifications |
| `specs/docker-compose-overlay/research/` | 4 research documents on compose semantics, resource limits, ps/exec, and config mapping |
| `specs/docker-compose-overlay/design.md` | Detailed design with architecture, components, data models, acceptance criteria |
| `specs/docker-compose-overlay/plan.md` | 8-step incremental implementation plan |

## Implementation Overview

Added Docker Compose as a third container runtime option alongside Docker and Apple Container. When enabled, AoE generates a runtime overlay YAML that defines the agent service and manages the full stack via `docker compose up/down/exec`.

## Files Changed

### New files
- `src/containers/compose.rs` -- `ComposeEngine` struct with overlay generation, lifecycle (up/down/is_running/exists), exec, and cleanup methods

### Modified files
- `src/session/config.rs` -- `Compose` variant on `ContainerRuntimeName`, `ComposeConfig` struct, `validate_compose_config()`
- `src/session/profile_config.rs` -- `ComposeConfigOverride`, merge logic in `apply_sandbox_overrides()`
- `src/session/mod.rs` -- Updated public exports
- `src/containers/mod.rs` -- Registered compose module, `Compose` variant handling in `get_container_runtime()` and `runtime_binary()`
- `src/session/instance.rs` -- `SandboxRuntime` enum, compose path in `get_container_for_instance()`, `ensure_compose_running()`, compose-aware `stop()`
- `src/session/builder.rs` -- Compose-aware cleanup in `cleanup_instance()`
- `src/cli/remove.rs` -- Compose-aware deletion (down --volumes + overlay cleanup)
- `src/cli/session.rs` -- Compose-aware is_running check in stop command
- `src/tui/deletion_poller.rs` -- Compose-aware background deletion
- `src/tui/settings/fields.rs` -- `Compose` option in runtime selector, `ComposeFiles` and `ComposeAgentService` fields with dynamic visibility

## Test Results

672 tests passing (24 new tests added). Clean clippy and fmt.

## Known Limitations

- **Container hooks**: `on_create`/`on_launch` hooks that run inside the container use `docker exec` directly. In compose mode, these will fail with a warning since the container name doesn't match. This can be addressed in a follow-up by making hook execution compose-aware.
- **Status poller**: Compose containers don't appear in the Docker-mode batch health check. Compose sessions fall through to tmux-based status detection, which works correctly but doesn't detect a dead compose container as fast as the Docker path.

## Configuration Example

```toml
[sandbox]
container_runtime = "compose"
default_image = "ghcr.io/njbrake/aoe-sandbox:latest"

[sandbox.compose]
compose_files = ["docker-compose.yml"]
agent_service = "aoe-agent"
```

## Next Steps

- Test end-to-end with a real Docker Compose setup
- Add compose-aware hook execution
- Consider compose-specific batch health checking for the status poller
