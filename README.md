# Agent of Empires (aoe)

A terminal session manager for Linux and MacOS using tmux to aid in management and monitoring of AI coding agents, written in Rust.

![Agent of Empires TUI](assets/tui.png)

## Installation

```bash
# Quick install (Linux & macOS)
curl -fsSL https://raw.githubusercontent.com/njbrake/agent-of-empires/main/scripts/install.sh | bash

# Homebrew
brew install njbrake/aoe/aoe # update via `brew update && brew upgrade aoe`

# Build from source
git clone https://github.com/njbrake/agent-of-empires && cd agent-of-empires && cargo build --release
```

## How It Works

Agent of Empires (aoe) is a wrapper around [tmux](https://github.com/tmux/tmux/wiki), the terminal multiplexer. Each AI coding session you create is actually a tmux session under the hood.

Once you attach to a session, you're working directly in tmux. Basic tmux knowledge helps:

| tmux Command | What It Does |
|--------------|--------------|
| `Ctrl+b d` | Detach from session (return to Agent of Empires) |
| `Ctrl+b [` | Enter scroll/copy mode |
| `Ctrl+b n` / `Ctrl+b p` | Next/previous window |

If you're new to tmux, the key thing to remember is `Ctrl+b d` to detach and return to the TUI.

## Features

- **TUI Dashboard** - Visual interface to manage all your AI coding sessions
- **Session Management** - Create, attach, detach, and delete sessions
- **Group Organization** - Organize sessions into hierarchical folders
- **Status Detection** - Automatic status detection for Claude Code and OpenCode
- **tmux Integration** - Sessions persist in tmux for reliability
- **Multi-profile Support** - Separate workspaces for different projects

## Quick Start

```bash
# Launch the TUI
aoe

# Or add a session directly from CLI
aoe add /path/to/project
```

## Using the TUI

### Launching

```bash
aoe           # Launch TUI with default profile
aoe -p work   # Launch with a specific profile
```

### Session Status Indicators

- 🟢 **Running** - Agent is actively processing
- 🟡 **Waiting** - Agent is waiting for input
- ⚪ **Idle** - Session is inactive
- 🔴 **Error** - An error was detected

## CLI Commands

```bash
# Session management
aoe add <path>              # Add a new session
aoe add . --title "my-proj" # Add with custom title
aoe list                    # List all sessions
aoe list --json             # List as JSON
aoe remove <id|title>       # Remove a session
aoe status                  # Show status summary

# Session lifecycle
aoe session start <id>      # Start a session
aoe session stop <id>       # Stop a session
aoe session restart <id>    # Restart a session
aoe session attach <id>     # Attach to a session
aoe session show <id>       # Show session details

# Groups
aoe group create <name>     # Create a group
aoe group list              # List groups
aoe group delete <name>     # Delete a group

# Profiles
aoe profile list            # List profiles
aoe profile create <name>   # Create a profile
aoe profile delete <name>   # Delete a profile

# Maintenance
aoe uninstall               # Uninstall Agent of Empires
```

## Configuration

### Profiles

Profiles let you maintain separate workspaces with their own sessions and groups. This is useful when you want to keep different contexts isolated—for example, work projects vs personal projects, or different client engagements.

```bash
aoe                 # Uses "default" profile
aoe -p work         # Uses "work" profile
aoe -p client-xyz   # Uses "client-xyz" profile
```

Each profile stores its own `sessions.json` and `groups.json`, so switching profiles gives you a completely different set of sessions.

### File Locations

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

# Build release binary
cargo build --release
```

## FAQ

### Using aoe with mobile SSH clients (Termius, Blink, etc.)

If you're connecting via SSH from a mobile app like Termius, you may encounter issues when attaching to sessions. The recommended approach is to run `aoe` inside a tmux session:

```bash
# Start a tmux session first
tmux new-session -s main

# Then run aoe inside it
aoe
```

When you attach to an agent session, tmux will switch to that session. To navigate back to `aoe` use the tmux command `Ctrl+b L` to switch to last session (toggle back to aoe)


## Acknowledgments

Inspired by [agent-deck](https://github.com/asheshgoplani/agent-deck) (Go + Bubble Tea).

## License

MIT License - see [LICENSE](LICENSE) for details.
