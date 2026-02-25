# Docker Compose Merge Semantics

## File Order

`docker compose -f base.yaml -f overlay.yaml` -- overlay wins on conflicts.

## Merge Rules by Field Type

| Category | Behavior | Examples |
|---|---|---|
| Single-value | Replace | `image`, `command`, `working_dir`, `mem_limit` |
| Keyed merge | Merge by key, overlay wins on conflict | `environment` (key=var name), `volumes` (key=container path) |
| Multi-value append | Concatenate both lists | `ports`, `expose`, `dns` |

### Volumes: merge by container mount path

Container-side path is the unique key. Same mount path in overlay replaces that entry. New paths are added.

```yaml
# base.yaml
services:
  web:
    volumes:
      - ./original:/foo
      - ./original:/bar

# overlay.yaml
services:
  web:
    volumes:
      - ./local:/bar     # replaces /bar mount
      - ./local:/baz     # added

# Result: /foo (kept), /bar (replaced), /baz (added)
```

### Environment: merge by variable name

```yaml
# base.yaml
services:
  web:
    environment:
      FOO: original
      BAR: original

# overlay.yaml
services:
  web:
    environment:
      BAR: local    # replaces
      BAZ: local    # added

# Result: FOO=original, BAR=local, BAZ=local
```

### Scalars: replace entirely

`command`, `working_dir`, `image` -- overlay value replaces base.

## Implication for AoE

Since AoE fully owns the agent service (user doesn't define it), merge semantics are mostly irrelevant -- the overlay is the only definition. The merge rules only matter if a user accidentally defines the same service name in their compose file (edge case, overlay wins).

## Sources

- https://docs.docker.com/compose/how-tos/multiple-compose-files/merge/
