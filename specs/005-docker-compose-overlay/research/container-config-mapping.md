# ContainerConfig to Compose Overlay Mapping

## ContainerConfig struct

```rust
pub struct ContainerConfig {
    pub working_dir: String,
    pub volumes: Vec<VolumeMount>,
    pub anonymous_volumes: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub cpu_limit: Option<String>,
    pub memory_limit: Option<String>,
}
```

## Field-by-field mapping

### working_dir

- Docker run: `-w /workspace/my-project`
- Compose: `working_dir: /workspace/my-project`

### volumes (Vec<VolumeMount>)

Sources: project mount, .gitconfig, .ssh, opencode config, 5 agent sandbox dirs, agent seed files, extra_volumes from config.

- Docker run: `-v /host:/container` or `-v /host:/container:ro`
- Compose:
  ```yaml
  volumes:
    - /host:/container
    - /host:/container:ro
  ```

### anonymous_volumes (Vec<String>)

From `volume_ignores` config. Expanded to full container paths.

- Docker run: `-v /workspace/project/node_modules` (no host path = anonymous)
- Compose:
  ```yaml
  volumes:
    - /workspace/project/node_modules
  ```

### environment (Vec<(String, String)>)

Sources: terminal vars (TERM, COLORTERM, etc.), config env keys, config env values, agent-specific vars, yolo mode flag, session extras.

- Docker run: `-e KEY=VALUE`
- Compose:
  ```yaml
  environment:
    KEY: VALUE
  ```

### cpu_limit (Option<String>)

- Docker run: `--cpus 2`
- Compose:
  ```yaml
  deploy:
    resources:
      limits:
        cpus: '2'
  ```

### memory_limit (Option<String>)

- Docker run: `-m 4g`
- Compose:
  ```yaml
  deploy:
    resources:
      limits:
        memory: 4g
  ```

## Additional fields in overlay (not from ContainerConfig)

- `image`: from `SandboxConfig.default_image`
- `command: sleep infinity`
- `stdin_open: true` + `tty: true` (equivalent of `-it` on docker run)

## Example overlay

```yaml
services:
  aoe-agent:
    image: ghcr.io/njbrake/aoe-sandbox:latest
    working_dir: /workspace/my-project
    command: sleep infinity
    stdin_open: true
    tty: true
    volumes:
      - /home/user/code/project:/workspace/my-project
      - /home/user/.gitconfig:/root/.gitconfig:ro
      - /home/user/.ssh:/root/.ssh:ro
      - /home/user/.claude/sandbox:/root/.claude
      - /home/user/.claude/sandbox/.claude.json:/root/.claude.json
      # ... more agent dirs ...
      - /workspace/my-project/node_modules
      - /workspace/my-project/target
    environment:
      TERM: xterm-256color
      COLORTERM: truecolor
      # ... more vars ...
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4g
```
