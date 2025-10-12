# Syspac - System Package Repository Manager

A Rust-based tool for managing Artix Linux package repositories hosted on GitHub.

## Overview

Syspac helps manage a GitHub-based Arch/Artix Linux package repository where packages are stored as git submodules. It provides functionality to:

- Detect which packages have changed between commits
- List all available packages in the repository
- Parse PKGBUILD files for version information
- Integrate with GitHub Actions for automated builds
- Preserve unchanged packages in repository releases

## Features

- **Modular Architecture**: Clean separation of concerns with dedicated modules for git operations, package discovery, and PKGBUILD parsing
- **Testable**: Unit tests for all major components
- **Flexible**: Works with both git submodules and direct package directories
- **CI/CD Ready**: Designed to integrate seamlessly with GitHub Actions workflows
- **Smart Release Management**: Preserves unchanged packages when updating releases
- **Slash Commands**: Trigger full rebuilds via GitHub comments

## Installation

### From Source

```bash
cargo build --release
```

The binary will be available at `target/release/syspac`.

## Usage

### Detect Changed Packages

Detect packages that have changed between commits:

```bash
# Compare against parent commit (HEAD^)
syspac detect-changes

# Compare against a specific ref
syspac detect-changes --base-ref main

# Get ALL packages (for full rebuild)
syspac detect-changes --all

# Output as JSON
syspac detect-changes --format json

# Specify repository path
syspac detect-changes --repo-path /path/to/repo
```

**Output formats:**
- `space` (default): Space-separated list of package names (e.g., "niri valent ly")
- `json`: JSON array of package names

### List All Packages

List all packages in the repository:

```bash
# Simple list
syspac list-packages

# With version information
syspac list-packages --verbose

# Specify repository path
syspac list-packages --repo-path /path/to/repo
```

### Get Package Version

Extract version information from a PKGBUILD:

```bash
# From a PKGBUILD file
syspac package-version /path/to/PKGBUILD

# From a package directory (will look for PKGBUILD inside)
syspac package-version /path/to/package-dir
```

## Architecture

### Module Structure

```
src/
├── main.rs          # CLI entry point and command handling
├── git.rs           # Git operations (diff, change detection)
├── package.rs       # Package discovery (submodules + directories)
└── pkgbuild.rs      # PKGBUILD parsing (version extraction)
```

### How It Works

1. **Package Discovery** (`package.rs`):
   - Scans git submodules for PKGBUILD files
   - Searches direct directories (up to 2 levels) for PKGBUILD files
   - Filters out common non-package directories (.git, target, node_modules, etc.)

2. **Change Detection** (`git.rs`):
   - Uses libgit2 to compare trees between commits
   - Maps changed files to their parent packages
   - Returns sorted list of unique changed packages

3. **PKGBUILD Parsing** (`pkgbuild.rs`):
   - Primary method: Sources PKGBUILD with bash (most reliable)
   - Fallback: Simple regex-based parser (for cases without bash)
   - Extracts pkgver and pkgrel variables

## GitHub Actions Integration

### Basic Integration

Replace the complex bash logic in your workflow with the Rust tool:

```yaml
- name: Detect changed packages
  id: changes
  run: |
    cargo build --release
    CHANGED=$(./target/release/syspac detect-changes)
    echo "packages=${CHANGED}" >> $GITHUB_OUTPUT
    echo "Changed packages: ${CHANGED}"
```

### Full Rebuild Support

The workflow automatically handles full rebuilds triggered by slash commands:

```yaml
- name: Detect changed packages
  id: changes
  run: |
    if [[ "${{ github.event_name }}" == "repository_dispatch" ]] && [[ "${{ github.event.action }}" == "rebuild-all" ]]; then
      CHANGED=$(./target/release/syspac detect-changes --all)
    else
      CHANGED=$(./target/release/syspac detect-changes)
    fi
    echo "packages=${CHANGED}" >> $GITHUB_OUTPUT
```

### Preserving Unchanged Packages

**Critical**: The workflow now preserves unchanged packages by downloading existing release assets before building:

```yaml
- name: Download existing release assets
  run: |
    if gh release view repository >/dev/null 2>&1; then
      cd repo/x86_64
      gh release download repository --pattern "*.pkg.tar.zst*"
      gh release download repository --pattern "syspac.db*"
      gh release download repository --pattern "syspac.files*"
    fi
```

This ensures that:
- Only changed packages are rebuilt
- Unchanged packages remain in the release
- The pacman database includes all packages (old + new)
- Users can always access all packages

See `.github/workflows/build-rust.yml.example` for the complete workflow.

## Slash Commands

You can trigger a full rebuild of all packages by commenting on a PR or issue:

```
/rebuild-all
```

This will:
1. Build ALL packages regardless of changes
2. Update the release with all packages (preserving everything)
3. Useful after dependency updates or toolchain changes

See [docs/SLASH_COMMANDS.md](docs/SLASH_COMMANDS.md) for detailed documentation.

## Development

### Running Tests

```bash
cargo test
```

### Adding New Functionality

The modular architecture makes it easy to extend:

1. **Add a new command**: Update `Commands` enum in `main.rs`
2. **Add git operations**: Extend `git.rs`
3. **Add package logic**: Extend `package.rs`
4. **Add PKGBUILD parsing**: Extend `pkgbuild.rs`

### Dependencies

- `anyhow`: Error handling
- `clap`: CLI argument parsing
- `git2`: Git operations via libgit2
- `serde` + `serde_json`: JSON serialization

## Migration from Shell Scripts

### Before (Shell)
```bash
# Complex bash script with git diff and submodule parsing
BASE_REF=$(git rev-parse HEAD^)
mapfile -t ALL_PKGS < <(git submodule foreach --quiet '...')
# ... 50+ lines of bash logic
```

### After (Rust)
```bash
syspac detect-changes
```

**Benefits:**
- Type safety and compile-time guarantees
- Better error handling
- Easier to test and maintain
- Consistent behavior across environments
- Clear separation of concerns
- Preserves unchanged packages in releases

See [MIGRATION.md](MIGRATION.md) for a complete migration guide.

## Key Improvements Over Shell Version

### 1. Package Preservation
- **Old**: Deleted and recreated release each time, losing unchanged packages
- **New**: Downloads existing packages before building, preserves everything

### 2. Full Rebuild Capability
- **Old**: Had to manually modify workflow or push all packages
- **New**: Simple `/rebuild-all` comment triggers full rebuild

### 3. Better Error Handling
- **Old**: Cryptic bash errors, silent failures
- **New**: Detailed error messages with context

### 4. Testability
- **Old**: Hard to test bash scripts
- **New**: Comprehensive unit and integration tests

### 5. Performance
- **Old**: Slow bash execution
- **New**: Fast compiled binary

## Usage Example

```bash
# Clone your repository
git clone https://github.com/yourusername/syspac.git
cd syspac

# Build the tool
cargo build --release

# List all packages with versions
./target/release/syspac list-packages --verbose
# Output:
# connman-resolvd: 1.2.0-1
# ly: 0.6.0-2
# niri: 0.1.0-1
# valent: 1.0.0-1

# Make some changes to a package
cd packages/niri
# ... edit files ...
git add .
git commit -m "Update niri to 0.1.1"
cd ../..

# Detect what changed
./target/release/syspac detect-changes
# Output: niri

# Get version of specific package
./target/release/syspac package-version packages/niri
# Output: 0.1.1-1

# Rebuild all packages (if needed)
./target/release/syspac detect-changes --all
# Output: connman-resolvd ly niri valent
```

## Repository Structure

```
syspac/
├── .github/
│   └── workflows/
│       ├── build-rust.yml.example    # Example build workflow
│       └── slash-commands.yml        # Slash command handler
├── build-container/
│   ├── Dockerfile                    # Build environment
│   └── entrypoint.sh                 # Build script
├── packages/                         # Git submodules
│   ├── niri/
│   ├── valent/
│   └── ...
├── src/
│   ├── main.rs                       # CLI interface
│   ├── git.rs                        # Git operations
│   ├── package.rs                    # Package discovery
│   └── pkgbuild.rs                   # PKGBUILD parsing
├── tests/
│   └── integration_tests.rs          # Integration tests
├── docs/
│   └── SLASH_COMMANDS.md             # Slash command docs
├── Cargo.toml                        # Dependencies
├── README.md                         # This file
├── MIGRATION.md                      # Migration guide
└── ARCHITECTURE.md                   # Technical details
```

## Troubleshooting

### Tool doesn't detect changes

**Problem**: `syspac detect-changes` returns empty

**Solution**: 
- Check if you're on a branch with commits: `git log`
- Try specifying base ref: `syspac detect-changes --base-ref main`
- Verify packages exist: `syspac list-packages`

### PKGBUILD parsing fails

**Problem**: `package-version` command fails

**Solution**:
- Ensure bash is available: `which bash`
- Check PKGBUILD syntax: `bash -n path/to/PKGBUILD`
- Try sourcing manually: `source path/to/PKGBUILD && echo $pkgver-$pkgrel`

### Packages missing after build

**Problem**: Old packages disappeared from release

**Solution**:
- Ensure workflow downloads existing assets (check logs)
- Verify "Download existing release assets" step succeeded
- Check for build failures in specific packages

## Contributing

Contributions are welcome! Please ensure:

1. Code is formatted with `cargo fmt`
2. All tests pass with `cargo test`
3. New functionality includes tests
4. Documentation is updated

## License

See LICENSE file for details.

## Support

- [GitHub Issues](https://github.com/yourusername/syspac/issues)
- [Documentation](docs/)
- [Architecture Guide](ARCHITECTURE.md)
