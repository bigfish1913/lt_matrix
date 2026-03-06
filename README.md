# ltmatrix

A high-performance, cross-platform long-time agent orchestrator rewritten in Rust from Python.

## Project Structure

```
ltmatrix/
├── src/                      # Main source code
│   ├── main.rs              # Binary entry point
│   ├── lib.rs               # Library entry point
│   ├── cli/                 # Command-line interface
│   │   ├── args.rs          # Argument parsing
│   │   └── command.rs       # Command handling
│   ├── config/              # Configuration management
│   │   ├── settings.rs      # Settings loader
│   │   ├── agent.rs         # Agent configuration
│   │   └── modes.rs         # Execution modes (fast/standard/expert)
│   ├── agent/               # Agent backend implementations
│   │   ├── backend.rs       # Agent trait definitions
│   │   ├── claude.rs        # Claude agent implementation
│   │   ├── pool.rs          # Agent pool management
│   │   └── session.rs       # Session management
│   ├── pipeline/            # Pipeline execution stages
│   │   ├── stage.rs         # Stage trait definitions
│   │   ├── generate.rs      # Generate stage
│   │   ├── assess.rs        # Assess stage
│   │   ├── execute.rs       # Execute stage
│   │   ├── test.rs          # Test stage
│   │   ├── verify.rs        # Verify stage
│   │   └── commit.rs        # Commit stage
│   ├── tasks/               # Task management
│   │   ├── task.rs          # Task definitions
│   │   ├── scheduler.rs     # Task scheduler
│   │   ├── dependency.rs    # Dependency graph
│   │   └── executor.rs      # Task executor
│   ├── git/                 # Git integration
│   │   ├── repository.rs    # Repository operations
│   │   ├── branch.rs        # Branch management
│   │   ├── commit.rs        # Commit operations
│   │   └── merge.rs         # Merge strategies
│   ├── memory/              # Project memory
│   │   ├── store.rs         # Memory storage
│   │   ├── memory.rs        # Memory structures
│   │   └── extractor.rs     # Memory extraction
│   ├── logging/             # Logging system
│   │   ├── logger.rs        # Logger implementation
│   │   ├── formatter.rs     # Output formatting
│   │   └── level.rs         # Log levels
│   └── progress/            # Progress tracking
│       ├── tracker.rs       # Progress tracker
│       ├── bar.rs           # Progress bars
│       └── reporter.rs      # Progress reporting
├── tests/                   # Integration tests
├── benches/                 # Benchmarks
├── examples/                # Example code
├── docs/                    # Documentation
│   ├── longtime.py          # Python reference implementation
│   ├── require.md           # Requirements document
│   ├── tasks/               # Task definitions
│   └── logs/                # Execution logs
└── Cargo.toml               # Project manifest
```

## Features

- Multi-agent backend support (Claude, OpenCode, KimiCode, Codex)
- 6-stage pipeline: Generate → Assess → Execute → Test → Verify → Commit
- Task dependency scheduling with parallel execution
- Automatic testing with failure recovery
- Git integration with per-task branching
- Project memory management
- Structured logging with multiple output formats
- Real-time progress tracking
- Cross-platform binary releases

## Installation

### From Cargo

```bash
cargo install ltmatrix
```

### From Release Binaries

Pre-built binaries are available for:
- **Windows** (x86_64, ARM64)
- **Linux** (x86_64, aarch64) - requires glibc 2.17+
- **macOS** (Intel, Apple Silicon)

See [Releases](https://github.com/bigfish/ltmatrix/releases) for downloads.

## Building from Source

### Prerequisites

- Rust 1.70+ toolchain
- For cross-compilation: `cargo-zigbuild` and Zig compiler

### Build Commands

```bash
# Build for host platform
cargo build --release

# Build for Linux (from any platform)
./build-linux.sh          # Unix/macOS
build-linux.bat           # Windows

# Build for macOS
cargo build --release --target x86_64-apple-darwin      # Intel
cargo build --release --target aarch64-apple-darwin      # Apple Silicon

# Build for Windows
cargo build --release --target x86_64-pc-windows-msvc    # x86_64
cargo build --release --target aarch64-pc-windows-msvc   # ARM64
```

### Cross-Compilation Notes

- **Linux builds** from Windows/macOS produce dynamically linked binaries (require glibc 2.17+)
- **Static Linux binaries** must be built on Linux using musl targets
- See [docs/LINUX_BUILD_REPORT.md](docs/LINUX_BUILD_REPORT.md) for detailed build information

## Usage

```bash
# Standard mode
ltmatrix "build a REST API"

# Fast mode
ltmatrix --fast "add error handling"

# Expert mode
ltmatrix --expert "implement authentication"

# Resume interrupted work
ltmatrix --resume

# Generate plan without execution
ltmatrix --dry-run "refactor database layer"
```

## License

MIT
