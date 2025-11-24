# ssh-tui

A Terminal User Interface (TUI) for managing SSH connections. Browse your SSH hosts from `~/.ssh/config` and connect with a single keypress.

## Features

- **Interactive TUI**: Browse SSH hosts using arrow keys or vim-style navigation (j/k)
- **Quick Connect**: Press Enter to connect to the selected host
- **Seamless Terminal Mode Switching**: Automatically drops the TUI, runs SSH in foreground, and restores the UI when done
- **Connection Status Display**: Shows connection results and errors in the UI footer
- **Configurable SSH Binary**: Use custom SSH binary path
- **Timeout Support**: Configure connection timeout
- **SSH Config Integration**: Automatically reads hosts from `~/.ssh/config`

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/ssh-tui`.

## Usage

Simply run the application:

```bash
ssh-tui
```

### Keyboard Controls

- **↑/k**: Move selection up
- **↓/j**: Move selection down
- **Enter**: Connect to the selected host
- **q**: Quit the application

## Configuration

Create a configuration file at `~/.config/ssh-tui/config.toml`:

```toml
# Path to SSH binary (default: "ssh")
ssh_binary = "/usr/bin/ssh"

# Connection timeout in seconds (default: 30)
timeout = 60
```

### Configuration Options

- `ssh_binary`: Path to the SSH executable (default: `"ssh"`)
- `timeout`: Connection timeout in seconds (default: `30`)

## How It Works

1. The application parses your `~/.ssh/config` file to find configured hosts
2. Displays hosts in an interactive list
3. When you press Enter on a host:
   - The TUI is temporarily suspended
   - The system `ssh` command is executed with the host alias
   - Your SSH config settings are used (leveraging host alias)
   - After SSH session ends, the TUI is restored
   - Connection status is displayed in the footer

## Testing

Run the test suite:

```bash
cargo test
```

The project includes:
- Unit tests for SSH config parsing
- Mock tests for SSH command execution
- Tests for UI state management
- Configuration tests

## Architecture

The application is structured into several modules:

- `main.rs`: Entry point and event loop handling
- `app.rs`: Application state and business logic
- `config.rs`: Configuration and SSH config parsing
- `ssh.rs`: SSH connection logic with command execution abstraction
- `ui.rs`: Terminal UI rendering

The SSH connection logic uses a trait-based approach (`CommandExecutor`) to allow for testing without actually executing SSH commands.

## Dependencies

- `ratatui`: Terminal UI framework
- `crossterm`: Cross-platform terminal manipulation
- `anyhow`: Error handling
- `serde` & `toml`: Configuration file parsing
- `dirs`: Home directory detection

## License

MIT
