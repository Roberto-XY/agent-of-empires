# Docker Compose v2 Resource Limits

## Correct Syntax (non-Swarm)

```yaml
services:
  myservice:
    deploy:
      resources:
        limits:
          cpus: '0.50'
          memory: 512M
```

## Key Facts

- `deploy.resources.limits` works without Swarm in Docker Compose v2 (Go rewrite).
- CPU limits confirmed working from v2.6.1+.
- Memory limits worked earlier.
- The old Python-based `docker-compose` (v1) ignored `deploy:` outside Swarm -- this is the source of most conflicting info online.

## Value Formats

- `cpus`: String. `'0.50'` = half a core, `'2.0'` = 2 cores. Must be quoted in YAML.
- `memory`: String with suffix. `128M`, `1G`, `512m`, `2g`. Case-insensitive suffix.

## Legacy Keys (deprecated but still work)

Top-level `mem_limit`, `cpus`, `cpu_shares`, `memswap_limit` -- from Compose file format v2.x. Deprecated in favor of `deploy.resources.limits`.

## Recommendation

Use `deploy.resources.limits` in the overlay for forward compatibility.

## Sources

- https://docs.docker.com/reference/compose-file/deploy/
- https://github.com/docker/compose/issues/9979
