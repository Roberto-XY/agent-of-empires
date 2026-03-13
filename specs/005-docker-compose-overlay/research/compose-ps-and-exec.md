# Docker Compose ps and exec

## `docker compose ps --format json`

### Output format (v2.21.0+): NDJSON

One JSON object per line, NOT a JSON array:

```json
{"ID":"f02a4ef","Name":"proj-web-1","Service":"web","State":"running","Health":"","ExitCode":0,"Publishers":null}
```

### Possible State values

`created` | `running` | `paused` | `restarting` | `removing` | `exited` | `dead`

### Checking a specific service

```bash
docker compose -f f.yaml -p proj ps --format json --status running aoe-agent
```

### Parsing in Rust

```rust
for line in output.lines() {
    if line.trim().is_empty() { continue; }
    let container: serde_json::Value = serde_json::from_str(line)?;
    if container["Service"] == "aoe-agent" && container["State"] == "running" {
        // running
    }
}
```

Empty output = no matching containers (not even `[]`).

---

## `docker compose exec`

### Syntax

```bash
docker compose -f compose.yaml -p myproject exec [OPTIONS] SERVICE COMMAND [ARGS...]
```

### Key difference from `docker exec`

TTY and interactive mode are ON by default. No `-it` needed.

### Flags

| Flag | Description |
|---|---|
| `-T, --no-tty` | Disable TTY (use for non-interactive/scripted) |
| `-e, --env KEY=VAL` | Set environment variables |
| `-w, --workdir DIR` | Working directory inside container |
| `-d, --detach` | Run in background |
| `-u, --user USER` | Run as user |

### Examples

```bash
# Interactive shell (TTY on by default)
docker compose -f f.yaml -p proj exec aoe-agent bash

# With workdir and env
docker compose -f f.yaml -p proj exec -w /workspace -e FOO=bar aoe-agent bash

# Non-interactive (scripted)
docker compose -f f.yaml -p proj exec -T aoe-agent ls -la
```

### Important for code

When stdin is not a terminal, pass `-T` to disable TTY. Otherwise errors or garbled output.

## Sources

- https://docs.docker.com/reference/cli/docker/compose/ps/
- https://docs.docker.com/reference/cli/docker/compose/exec/
