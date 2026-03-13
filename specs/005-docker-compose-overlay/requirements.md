# Requirements

## Questions & Answers

### Q1: Should Compose mode support multiple agent services, or is it strictly one agent service per compose file?

The rough idea mentions a single `compose_agent_service` config field pointing to one service. But a user's compose file might have multiple services (database, redis, etc.). Is AoE always targeting exactly one service in the overlay, and the other services are just "along for the ride" (started by compose but not managed by AoE)?

**A1:** The whole compose stack is managed by AoE (lifecycle: up/down). But only the one agent service gets the overlay injection (volumes, env vars, resource limits). Other services in the compose file are left as-is.

---

### Q2: Where does the user specify the path to their compose file?

Currently `SandboxConfig` has no field for a compose file path. The user needs to tell AoE where their `docker-compose.yml` lives. Should this be:
- A new config field (e.g., `compose_file`) in `SandboxConfig`, expected to be an absolute path or relative to the project root?
- Or does AoE auto-discover it by looking for `docker-compose.yml` / `compose.yml` in the project root?

**A2:** A new config field. Preference for structuring config so invalid state is unrepresentable -- compose config shouldn't pollute other runtimes' state space. Open question on whether the refactoring cost is worth it vs. a simpler flat approach.

---

### Q3: How should the config be structured to separate runtime concerns?

Currently `SandboxConfig` is a flat struct with a `container_runtime: ContainerRuntimeName` enum (Docker or AppleContainer). Adding Compose as a third variant is one option, but your preference is to avoid invalid states (e.g., `compose_file` set when runtime is Docker).

Two approaches:

**A) Tagged enum approach** -- `ContainerRuntimeName` becomes an enum with associated data:
```
container_runtime:
  Compose:
    compose_file: "./docker-compose.yml"
    agent_service: "agent"
```
This makes invalid states impossible but requires significant refactoring of config loading, TUI settings fields, profile overrides, and serialization.

**B) Nested section approach** -- Keep `container_runtime` as a simple enum with a new `Compose` variant, and add a `[sandbox.compose]` sub-table that's only read when `container_runtime = "Compose"`:
```toml
[sandbox]
container_runtime = "Compose"

[sandbox.compose]
compose_file = "./docker-compose.yml"
agent_service = "agent"
```
Less refactoring (flat fields still work, compose fields are just a nested struct validated at runtime), but invalid state is representable (compose fields set when runtime != Compose).

Which direction do you prefer?

**A3:** Option B -- nested section approach. Add a `Compose` variant to `ContainerRuntimeName` and a `[sandbox.compose]` sub-table. Validate at runtime that compose fields are present when runtime = Compose.

---

### Q4: Should the overlay include the `image` and `command` (sleep infinity) for the agent service, or must the user define those in their compose file?

Currently with `docker run`, AoE controls the image (`default_image` config) and the command (`sleep infinity`). In compose mode, the user defines the agent service themselves. Does the user own image + command entirely, and AoE only injects volumes/env/working_dir/resource limits? Or should the overlay also be able to override image and/or command?

**A4:** All existing AoE settings (image, command, etc.) should be written into the overlay. But since Docker Compose overlays merge with the base file, if the user's compose file already defines `image`, `command`, etc. for the agent service, the user's values take priority (compose last-file-wins semantics). So AoE generates a full overlay, but the user's compose file can override any of it.

Wait -- actually compose overlay order matters. If AoE runs `-f user.yaml -f overlay.yaml`, the overlay wins. To let the user's compose take priority, we'd need the reverse order or a different approach. Let me clarify this in the next question.

---

### Q5: Compose file ordering -- who wins on conflicts?

With `docker compose -f user.yaml -f overlay.yaml`, the **overlay wins** on conflicts (last file takes precedence). But you said the user's compose should take priority.

Two options:
- **A)** Reverse the order: `-f overlay.yaml -f user.yaml` -- user always wins, but then the user can accidentally override AoE's volume mounts and env vars, potentially breaking agent functionality.
- **B)** Keep `-f user.yaml -f overlay.yaml` (overlay wins), but have AoE be smart about what it puts in the overlay -- skip fields the user already defined in their compose file (parse the user YAML first, only inject what's missing).

Which approach?

**A5:** Cleaner responsibility split needed. AoE should fully own the agent service definition in the overlay. The user's compose file doesn't need to define the agent service at all -- it just has their own services (db, redis, etc.). AoE's overlay adds/defines the complete agent service from scratch using all existing AoE settings. `-f user.yaml -f overlay.yaml` order, no ambiguity about who owns what. If the user does happen to define the same service name, overlay wins, but that's an edge case not the expected flow.

---

### Q6: Does the user need to specify `compose_agent_service` or can AoE use a fixed name?

Since AoE fully owns the agent service definition in the overlay, the service name is AoE's to decide. Should it:
- Always be a fixed name like `aoe-agent`?
- Or still be configurable via a config field (in case the user wants other services to `depends_on` it by a specific name)?

**A6:** Configurable, default to `aoe-agent`. Goes in `[sandbox.compose]` as `agent_service = "aoe-agent"`.

---

### Q7: Should `compose_file` be relative to the project root or the AoE config directory?

The user's compose file likely lives in their project repo (next to their code). Should `compose_file` be resolved relative to the project root (the directory AoE is launched against), or should it require an absolute path?

**A7:** Relative to the project base path.

---

### Q8: How should `docker compose exec` work for agent interactions?

Currently AoE builds an exec command like `docker exec -it <container_name> <cmd>`. In Compose mode, this becomes `docker compose exec`. But `docker compose exec` requires the project context (the `-f` flags and `-p` project name) to know which stack to target.

Should AoE reconstruct the full compose command each time (`docker compose -f user.yaml -f overlay.yaml -p aoe-{session-id} exec aoe-agent <cmd>`), or should it store the compose context (file paths, project name) in `SandboxInfo` so it can be reused across process restarts?

**A8:** Same pattern as Docker mode -- derive everything from session ID + current config. Project name is deterministic (`aoe-{session_id_prefix}`), compose file comes from config, overlay path is at a known location (`{app_dir}/compose-overlays/{project_name}.override.yaml`). No extra state to persist.

---

### Q9: How should the Compose engine integrate with the existing `ContainerRuntimeInterface` trait?

Currently there's a `ContainerRuntimeInterface` trait with methods like `create_container`, `start`, `stop`, `remove`, `exec`, `exec_command`, `exists`, `is_running`. Docker and AppleContainer both implement this via `RuntimeBase`.

Compose has fundamentally different semantics:
- `create` = generate overlay YAML + `docker compose up -d` (starts the whole stack, not just one container)
- `exec` = `docker compose -f ... -p ... exec <service> <cmd>` (targets a service name, not a container name)
- `remove` = `docker compose down` + delete overlay file
- `exists`/`is_running` = `docker compose ps` to check service status

Should Compose implement the same `ContainerRuntimeInterface` trait (adapting the methods to compose semantics), or does it need a different abstraction?

**A9:** Separate `ComposeEngine` abstraction. The semantics are too different to share `ContainerRuntimeInterface` -- different inputs (file paths + project name vs container name), different lifecycle (stack vs single container), different exec context. Since Docker and Compose are mutually exclusive, there's no polymorphism benefit. Dispatch at call sites in `instance.rs` via match on `container_runtime`.

---

### Q10: What YAML serialization approach for the overlay?

AoE needs to generate a valid Docker Compose YAML file. Options:
- **A)** Use a YAML library (e.g., `serde_yaml`) with typed structs representing the compose schema
- **B)** Use a YAML library with untyped `serde_yaml::Value` / map construction
- **C)** Simple string templating (format! / handlebars) since the overlay structure is predictable and narrow

The overlay is always the same shape (one service with volumes, env, working_dir, resource limits, image, command). It won't grow into a general-purpose compose generator. Which approach?

**A10:** Option C -- simple string templating. The overlay shape is narrow and predictable, no need for a YAML library dependency.

---

### Q11: Should the `compose_file` config field support multiple files?

Some users split their compose setup across multiple files (e.g., `docker-compose.yml` + `docker-compose.db.yml`). Should `compose_file` be:
- A single path (string) -- simpler, covers the common case
- A list of paths -- more flexible, AoE passes all of them as `-f` flags before its own overlay

**A11:** List of paths. AoE passes all as `-f` flags before its own overlay `-f`.

---

### Q12: How should Compose mode handle image pulling?

In Docker mode, AoE checks if the image exists locally and pulls it if needed before `docker run`. In Compose mode, `docker compose up -d` handles image pulling automatically for all services in the stack.

Should AoE:
- Let `docker compose up` handle all image pulling (simpler, no special logic needed)
- Pre-pull the agent image before `up` to give better progress feedback in the TUI

**A12:** Let `docker compose up` handle pulling. No pre-pull. Surface errors in the TUI if pulling or startup fails.

---

### Q13: Error handling -- what if the user's compose file is invalid or the stack fails to start?

In Docker mode, AoE catches specific errors (permission denied, daemon not running, image not found). For Compose, additional failure modes exist:
- User's compose file has syntax errors
- A non-agent service fails to start (e.g., db port conflict)
- The agent service name in the overlay conflicts with something unexpected

Should AoE attempt to parse/validate the user's compose file before running `up`, or just let `docker compose up` fail and surface the stderr?

**A13:** No pre-validation. Let `docker compose up` fail and surface the stderr in the TUI.

---

### Q14: Should `docker compose down` remove volumes (`--volumes` flag)?

On cleanup (session deletion), AoE runs `docker compose down`. Currently Docker mode uses `docker rm -f -v` which removes anonymous volumes. Should `docker compose down` also use `--volumes` to remove anonymous volumes, or leave them for the user to manage?

**A14:** Yes, use `--volumes` to match Docker mode behavior.

---

### Q15: Should Compose mode be available on macOS with Apple Container runtime?

Currently `ContainerRuntimeName` has Docker and AppleContainer variants. Compose mode uses Docker Compose, which requires the Docker engine. Should Compose be:
- Docker-only (Compose variant is only valid when Docker is the underlying engine)
- Or could it theoretically work with Apple Container too?

**A15:** Docker-only. Compose variant requires the Docker engine.

---

### Q16: How should the TUI settings surface Compose configuration?

Currently the sandbox settings have fields for all `SandboxConfig` options. With the nested `[sandbox.compose]` section, the Compose-specific fields (`compose_file` list, `agent_service`) need to appear in the TUI.

Should they:
- Always be visible in the sandbox settings (grayed out / disabled when runtime != Compose)
- Only appear when runtime is set to Compose (dynamically shown/hidden)

**A16:** Only show Compose fields when runtime is set to Compose. Dynamically hidden otherwise.

---

### Q17: Should existing shared settings (extra_volumes, environment, environment_values, volume_ignores, mount_ssh, cpu_limit, memory_limit) still apply in Compose mode?

These settings currently feed into `build_container_config()` for Docker mode. In Compose mode, the same data would go into the overlay YAML. Should all of these shared settings carry over to Compose mode, or are some irrelevant (since the user can define them directly in their compose file)?

**A17:** All shared settings carry over. `build_container_config()` output feeds into the overlay template the same way it feeds into `docker run` args.

---

### Q18: Does AoE need to detect whether `docker compose` (v2) is available, or can it assume it?

Docker Compose v2 is a plugin (`docker compose`), while v1 was a separate binary (`docker-compose`). Should AoE:
- Only support v2 (`docker compose` subcommand)
- Fall back to v1 (`docker-compose`) if v2 is not available
- Check availability and error with a clear message if missing

**A18:** Only support v2 (`docker compose`). Check availability on startup when runtime = Compose, error with clear message if missing.

---

*Requirements clarification complete.*
