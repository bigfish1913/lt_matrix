# Man Page Generation

ltmatrix includes built-in support for generating Unix manual pages (man pages) for all commands and subcommands.

## Quick Start

### Generate Man Pages

```bash
# Generate man pages to default directory (./man)
ltmatrix man

# Generate man pages to custom directory
ltmatrix man --output /usr/local/share/man/man1
```

### View Man Pages

```bash
# After generating man pages
man ./man/ltmatrix.1

# After installing system-wide
man ltmatrix
```

### Install System-Wide

```bash
# Linux/macOS
sudo cp man/*.1 /usr/local/share/man/man1/

# Or use the provided script
bash scripts/generate_man_pages.sh
```

## Generated Man Pages

The following man pages are generated:

| Man Page | Description |
|----------|-------------|
| `ltmatrix(1)` | Main ltmatrix command and options |
| `ltmatrix-release(1)` | Release build subcommand |
| `ltmatrix-completions(1)` | Shell completions subcommand |
| `ltmatrix-man(1)` | Man page generation subcommand |

## Man Page Sections

Each man page includes the following sections:

- **NAME** - Command name and brief description
- **SYNOPSIS** - Command syntax and usage
- **DESCRIPTION** - Detailed description
- **OPTIONS** - All available options
- **EXAMPLES** - Usage examples (for main command)
- **VERSION** - Version information
- **AUTHORS** - Author information

## Integration with Build Process

### Automated Generation

Man pages can be automatically generated during the build process:

```bash
# Generate as part of release build
cargo build --release
./target/release/ltmatrix man --output dist/man
```

### Distribution Packaging

Include man pages in release packages:

```bash
# Create distribution directory structure
mkdir -p dist/ltmatrix-0.1.0/man

# Copy man pages
cp target/man/*.1 dist/ltmatrix-0.1.0/man/

# Package
tar czf ltmatrix-0.1.0.tar.gz dist/ltmatrix-0.1.0/
```

## Programmatic Usage

Man pages can be generated programmatically using the Rust API:

```rust
use ltmatrix::man::generate_man_pages;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let output_dir = PathBuf::from("./man");
    generate_man_pages(&output_dir)?;
    Ok(())
}
```

## Man Page Validation

Generated man pages are validated to ensure:

- Proper roff format (`.TH`, `.SH`, `.TP` macros)
- Required sections present (NAME, SYNOPSIS, DESCRIPTION)
- Valid content structure

Run tests to validate:

```bash
cargo test --test man_page_test
```

## Examples

### Basic Usage

```bash
$ ltmatrix man
ltmatrix - Man Page Generation
Output directory: ./man

Generated man pages:
  - ltmatrix-completions.1
  - ltmatrix-man.1
  - ltmatrix-release.1
  - ltmatrix.1

To view a man page, run:
  man ./man/ltmatrix.1
```

### Custom Output Directory

```bash
$ ltmatrix man --output /tmp/myman
ltmatrix - Man Page Generation
Output directory: /tmp/myman
...
```

## Troubleshooting

### Man Page Not Found

If `man ltmatrix` doesn't work after installation:

1. Check man pages are installed:
   ```bash
   ls /usr/local/share/man/man1/ltmatrix*
   ```

2. Update man database:
   ```bash
   sudo mandb  # Linux
   ```

3. Try full path:
   ```bash
   man -M /usr/local/share/man ltmatrix
   ```

### Permission Denied

If you get permission errors:

```bash
# Install to user directory instead
mkdir -p ~/.local/share/man/man1
cp man/*.1 ~/.local/share/man/man1/

# Add to MANPATH in ~/.bashrc or ~/.zshrc
echo "export MANPATH=$HOME/.local/share/man:$MANPATH" >> ~/.bashrc
```

## See Also

- [CLI Reference](./cli.md) - Complete command-line interface documentation
- [Configuration Reference](./config.md) - Configuration file documentation
- [Examples](../examples/) - Example usage
