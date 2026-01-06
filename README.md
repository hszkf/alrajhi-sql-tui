# Alrajhi Bank SQL Studio

High-performance SQL Server Terminal UI built with Rust for Alrajhi Bank.

## Features

- **Beautiful TUI** - Professional corporate design with Alrajhi Bank branding
- **Fast & Efficient** - Built in Rust for maximum performance
- **SQL Syntax Highlighting** - Color-coded SQL keywords, strings, and numbers
- **Schema Explorer** - Browse tables, views, and stored procedures
- **Query History** - Persistent history with timestamps
- **Results Table** - Scrollable with row numbers, type indicators, NULL highlighting
- **Tabbed Results** - View Data, Columns info, and Query Stats
- **Export** - CSV, JSON, and INSERT statement export
- **Mouse Support** - Scroll with mouse wheel in all panels

## Quick Install

### Prerequisites
- Rust toolchain (1.70+): https://rustup.rs/

### Install & Run

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/alrajhi-sql-tui.git
cd alrajhi-sql-tui

# Build release version
cargo build --release

# Run the application
./target/release/alrajhi_sql_tui
```

### Update to Latest Version

```bash
cd alrajhi-sql-tui
git pull
cargo build --release
./target/release/alrajhi_sql_tui
```

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit application |
| `F1` | Toggle help popup |
| `Ctrl+Tab` | Next panel |
| `Shift+Tab` | Previous panel |

### Query Editor
| Key | Action |
|-----|--------|
| `Enter` | Execute query |
| `Shift+Enter` | New line |
| `Tab` | Insert 4 spaces (indent) |
| `Ctrl+F` | Format SQL |
| `F5` | Execute query |
| `Esc` | Clear query |
| Arrow keys | Move cursor |

### Results Panel
| Key | Action |
|-----|--------|
| `1` / `2` / `3` | Switch to Data/Columns/Stats tab |
| `Tab` | Cycle through tabs |
| `j/k` or `Up/Down` | Navigate rows |
| `h/l` or `Left/Right` | Navigate columns |
| `PageUp/PageDown` | Fast scroll (20 rows) |
| `Home/End` | First/Last row |
| `Ctrl+Y` | Copy cell value |
| `Ctrl+E` | Export to CSV |
| `Ctrl+S` | Export to JSON |
| `Ctrl+I` | Copy row as INSERT |
| Mouse scroll | Scroll through results |

### Schema Explorer
| Key | Action |
|-----|--------|
| `Up/Down` | Navigate |
| `Enter/Space` | Expand folder / Insert table name |
| Mouse scroll | Scroll through schema |

### History Panel
| Key | Action |
|-----|--------|
| `Up/Down` | Navigate |
| `Enter` | Load query |
| Mouse scroll | Scroll through history |

## Configuration

Edit `src/db/connection.rs` to change database settings:

```rust
host: "10.200.224.42"
port: 1433
database: "Staging"
username: "ssis_admin"
password: "your_password"
```

## Project Structure

```
src/
├── main.rs           # Entry point
├── app/              # Application state & handlers
│   ├── mod.rs
│   ├── state.rs      # App state
│   ├── handlers.rs   # Keyboard & mouse handlers
│   └── history.rs    # Query history
├── db/               # Database layer
│   ├── mod.rs
│   ├── connection.rs # SQL Server connection
│   ├── query.rs      # Query execution with DATE handling
│   └── schema.rs     # Schema explorer
└── ui/               # User interface
    ├── mod.rs
    ├── theme.rs      # Alrajhi Bank colors
    ├── layout.rs     # Panel layout
    └── widgets.rs    # UI components with tabs
```

## Technologies

- **ratatui 0.26** - Terminal UI framework
- **tiberius** - SQL Server driver (TDS protocol)
- **tokio** - Async runtime
- **crossterm 0.27** - Cross-platform terminal handling
- **arboard** - Clipboard support

## Troubleshooting

### Build Errors
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Connection Issues
- Verify SQL Server is accessible on port 1433
- Check firewall settings
- Verify credentials in connection.rs

---
Built with Rust for Alrajhi Bank IT Team
