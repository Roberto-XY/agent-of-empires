# Contributing to Agent of Empires

Thank you for your interest in contributing to Agent of Empires! This document provides guidelines and information for contributors.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/agent-of-empires.git
   cd agent-of-empires
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/nbrake/agent-of-empires.git
   ```

## Development Setup

### Prerequisites

- Rust 1.75 or later
- tmux
- Cargo

### Building

```bash
cargo build --release    # Build optimized binary to ./target/release/agent-of-empires
cargo test               # Run tests
cargo clippy             # Run linter
cargo fmt                # Format code
```

### Running Locally

```bash
cargo run --release      # Run directly
# or
./target/release/agent-of-empires
```

## Making Changes

### Branch Naming

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation changes
- `refactor/description` - Code refactoring

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add support for custom commands
fix: resolve status detection for OpenCode
docs: update installation instructions
refactor: simplify group management logic
```

### Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` to check for issues
- Follow existing code patterns
- Add tests for new functionality

## Pull Request Process

1. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes** and commit them

3. **Keep your branch updated**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

4. **Push to your fork**:
   ```bash
   git push origin feature/my-feature
   ```

5. **Open a Pull Request** on GitHub

### PR Guidelines

- Provide a clear description of the changes
- Reference any related issues
- Ensure all tests pass
- Update documentation if needed

## Reporting Issues

### Bug Reports

Include:
- Agent of Empires version (`agent-of-empires --version`)
- Operating system and version
- tmux version (`tmux -V`)
- Steps to reproduce
- Expected vs actual behavior
- Any error messages or logs

### Feature Requests

- Describe the use case
- Explain why existing features don't solve it
- Provide examples if possible

## Project Structure

```
agent-of-empires/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library root
│   ├── cli/              # CLI command handlers
│   ├── tui/              # TUI components (ratatui)
│   ├── session/          # Session & group management
│   ├── tmux/             # tmux integration, status detection
│   ├── mcppool/          # MCP server pooling
│   ├── platform/         # Platform detection
│   └── update/           # Self-update mechanism
├── Cargo.toml            # Dependencies
└── README.md
```

## Testing

- Add tests for new functionality
- Run the full test suite: `cargo test`
- Tests should be deterministic and not depend on external state

### Debug Mode

Enable debug logging:
```bash
RUST_LOG=agent_of_empires=debug agent-of-empires
```

## Questions?

Feel free to open an issue for questions or discussion.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
