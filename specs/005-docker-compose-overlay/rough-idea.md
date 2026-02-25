# Docker Compose Overlay

## Rough Idea

When Docker Compose is enabled, AoE should not create a separate agent container via `docker run`. Instead, the user defines an agent service in their own compose file, and AoE generates a runtime overlay YAML that injects volumes, env vars, `working_dir`, and resource limits into that service. AoE then runs `docker compose -f user.yaml -f overlay.yaml -p aoe-{session-id} up -d` and uses `docker compose exec` instead of `docker exec` for all agent interactions.

## Key Points

- User defines the agent service in their compose file (image, tty, etc.). AoE knows which service via a new `compose_agent_service` config field.
- AoE generates a runtime overlay at `{app_dir}/compose-overlays/{project_name}.override.yaml` containing all the volumes (workspace, agent configs, gitconfig, ssh), environment variables, anonymous volumes (`volume_ignores`), and resource limits that `build_container_config()` currently computes for `docker run`.
- Docker engine and Compose engine are mutually exclusive. When `compose_enabled=true`, no `docker run`/`docker exec` is used. When `compose_enabled=false`, no compose code runs.
- Networking is automatic -- compose puts all services on the same network, eliminating the need for network discovery or `docker network connect`.
- Overlay persists in the app dir (not a tempfile) so `docker compose exec` calls work after the initial `up`, across process restarts within a session.
- Cleanup: session deletion calls `docker compose down`, then deletes the overlay file.
