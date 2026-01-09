# Agent of Empires

A terminal session manager for AI coding agents, written in Rust.

## Features

- **TUI Dashboard** - Visual interface to manage all your AI coding sessions
- **Session Management** - Create, attach, detach, and delete sessions
- **Group Organization** - Organize sessions into hierarchical folders
- **Status Detection** - Automatic status detection for Claude, Gemini, OpenCode, and Codex
- **tmux Integration** - Sessions persist in tmux for reliability
- **MCP Server Management** - Configure and manage Model Context Protocol servers
- **Multi-profile Support** - Separate workspaces for different projects

## Requirements

- **tmux** - Required for session management
  - macOS: `brew install tmux`
  - Ubuntu/Debian: `sudo apt install tmux`

## Building

```bash
cargo build --release
```

The binary will be at `target/release/agent-of-empires`.

## Quick Start

```bash
# Launch the TUI
./target/release/agent-of-empires

# Or add a session directly from CLI
./target/release/agent-of-empires add /path/to/project
```

## Using the TUI

### Launching

```bash
agent-of-empires           # Launch TUI with default profile
agent-of-empires -p work   # Launch with a specific profile
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| **Navigation** | |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Collapse group |
| `l` / `→` | Expand group |
| `g` | Go to top |
| `G` | Go to bottom |
| `PageUp` / `PageDown` | Page navigation |
| **Actions** | |
| `Enter` | Attach to selected session |
| `n` | Create new session |
| `d` | Delete selected session |
| `r` / `F5` | Refresh session list |
| **Other** | |
| `/` | Search sessions |
| `?` | Toggle help overlay |
| `q` / `Ctrl+c` | Quit TUI |

### Attaching and Detaching from Sessions

1. **Attach to a session**: Select a session and press `Enter`
   - The TUI will temporarily exit and you'll be connected to the tmux session

2. **Detach from a session**: Press `Ctrl+b` then `d`
   - This is tmux's standard detach sequence
   - You'll return to the Agent of Empires TUI

3. **Alternative detach** (if already in tmux): The session will be switched, use `Ctrl+b d` to return

### Session Status Indicators

- 🟢 **Running** - Agent is actively processing
- 🟡 **Waiting** - Agent is waiting for input
- ⚪ **Idle** - Session is inactive
- 🔴 **Error** - An error was detected

## CLI Commands

```bash
# Session management
agent-of-empires add <path>              # Add a new session
agent-of-empires add . --title "my-proj" # Add with custom title
agent-of-empires list                    # List all sessions
agent-of-empires list --json             # List as JSON
agent-of-empires remove <id|title>       # Remove a session
agent-of-empires status                  # Show status summary

# Session lifecycle
agent-of-empires session start <id>      # Start a session
agent-of-empires session stop <id>       # Stop a session
agent-of-empires session restart <id>    # Restart a session
agent-of-empires session attach <id>     # Attach to a session
agent-of-empires session show <id>       # Show session details

# Groups
agent-of-empires group create <name>     # Create a group
agent-of-empires group list              # List groups
agent-of-empires group delete <name>     # Delete a group

# Profiles
agent-of-empires profile list            # List profiles
agent-of-empires profile create <name>   # Create a profile
agent-of-empires profile delete <name>   # Delete a profile

# MCP servers
agent-of-empires mcp list                # List configured MCP servers
agent-of-empires mcp attach <name>       # Attach MCP to current session
agent-of-empires mcp detach <name>       # Detach MCP from current session

# Maintenance
agent-of-empires update                  # Check for updates
agent-of-empires uninstall               # Uninstall Agent of Empires
```

## Configuration

Configuration is stored in `~/.agent-of-empires/`:

```
~/.agent-of-empires/
├── config.toml           # Global configuration
├── profiles/
│   └── default/
│       ├── sessions.json # Session data
│       └── groups.json   # Group structure
└── logs/                 # Session logs
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `AGENT_OF_EMPIRES_PROFILE` | Default profile to use |
| `AGENT_OF_EMPIRES_DEBUG` | Enable debug logging |

## Development

```bash
# Check code
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Run in debug mode
AGENT_OF_EMPIRES_DEBUG=1 cargo run
```

## Dependencies

Key dependencies:
- `ratatui` + `crossterm` - TUI framework
- `clap` - CLI argument parsing
- `serde` + `serde_json` + `toml` - Serialization
- `tokio` - Async runtime
- `notify` - File system watching
- `reqwest` - HTTP client for updates

## License

MIT License - see [LICENSE](LICENSE) for details.
