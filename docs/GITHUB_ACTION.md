# Syspac GitHub Action

A reusable GitHub Action that sets up the syspac package management tool in your workflows.

## Features

- **Fast Setup**: Downloads pre-built binaries instead of compiling
- **Smart Caching**: Caches binaries for even faster subsequent runs
- **Fallback Build**: Automatically builds from source if binary not available
- **Version Control**: Pin to specific versions or use latest
- **Zero Dependencies**: Statically-linked binaries (musl variant available)

## Usage

### Basic Usage

```yaml
name: My Workflow

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Syspac
        uses: hazayan/syspac@main
      
      - name: Use Syspac
        run: syspac detect-changes --paths
```

### With Specific Version

```yaml
- name: Setup Syspac
  uses: hazayan/syspac@main
  with:
    version: v0.2.0
```

### Build from Source

```yaml
- name: Setup Syspac (from source)
  uses: hazayan/syspac@main
  with:
    version: build
```

### With Custom GitHub Token

```yaml
- name: Setup Syspac
  uses: hazayan/syspac@main
  with:
    github-token: ${{ secrets.CUSTOM_TOKEN }}
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `version` | Version to use (`latest`, `vX.Y.Z`, or `build`) | No | `latest` |
| `github-token` | GitHub token for downloading releases | No | `${{ github.token }}` |

### Version Options

- **`latest`** (default): Downloads the most recent release
- **`v0.2.0`**: Downloads a specific version
- **`build`**: Compiles from source (requires Cargo.toml in repo)

## Outputs

| Output | Description |
|--------|-------------|
| `syspac-path` | Full path to the syspac binary |
| `syspac-version` | Version of syspac that was installed |

### Using Outputs

```yaml
- name: Setup Syspac
  id: syspac
  uses: hazayan/syspac@main

- name: Show version
  run: echo "Using syspac ${{ steps.syspac.outputs.syspac-version }}"

- name: Use specific path
  run: ${{ steps.syspac.outputs.syspac-path }} detect-changes
```

## Complete Examples

### Example 1: Package Build Workflow

```yaml
name: Build Packages

on:
  push:
    branches: [main]

jobs:
  detect-changes:
    runs-on: ubuntu-latest
    outputs:
      packages: ${{ steps.changes.outputs.packages }}
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
      
      - name: Setup Syspac
        uses: hazayan/syspac@main
      
      - name: Detect changed packages
        id: changes
        run: |
          CHANGED=$(syspac detect-changes --paths)
          echo "packages=$CHANGED" >> $GITHUB_OUTPUT
  
  build:
    needs: detect-changes
    if: needs.detect-changes.outputs.packages != ''
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build packages
        run: |
          for pkg in ${{ needs.detect-changes.outputs.packages }}; do
            echo "Building $pkg"
            # Build logic here
          done
```

### Example 2: Package Listing

```yaml
name: List Packages

on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly

jobs:
  list:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
      
      - name: Setup Syspac
        uses: hazayan/syspac@main
      
      - name: List all packages
        run: syspac list-packages --verbose
      
      - name: Export as JSON
        run: syspac list-packages --format json > packages.json
      
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: package-list
          path: packages.json
```

### Example 3: Matrix Build

```yaml
name: Matrix Build

on: [push]

jobs:
  prepare:
    runs-on: ubuntu-latest
    outputs:
      packages: ${{ steps.packages.outputs.list }}
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
      
      - name: Setup Syspac
        uses: hazayan/syspac@main
      
      - name: Get package list as JSON
        id: packages
        run: |
          PACKAGES=$(syspac detect-changes --all --format json)
          echo "list=$PACKAGES" >> $GITHUB_OUTPUT
  
  build:
    needs: prepare
    runs-on: ubuntu-latest
    strategy:
      matrix:
        package: ${{ fromJson(needs.prepare.outputs.packages) }}
    steps:
      - uses: actions/checkout@v3
      
      - name: Build ${{ matrix.package }}
        run: echo "Building ${{ matrix.package }}"
```

## Performance Comparison

### Before (Building from Source)

```yaml
- name: Install Rust
  uses: actions-rs/toolchain@v1
  # ~30 seconds

- name: Build syspac
  run: cargo build --release
  # ~2-3 minutes (first time)
  # ~30-60 seconds (with cache)

Total: ~2.5-3.5 minutes (first time), ~1-1.5 minutes (cached)
```

### After (Using Action)

```yaml
- name: Setup Syspac
  uses: hazayan/syspac@main
  # ~5-10 seconds (download)
  # ~1 second (cached)

Total: ~5-10 seconds (first time), ~1 second (cached)
```

**Improvement**: ~95% faster on first run, ~99% faster with cache!

## How It Works

1. **Check Version**: Determines which version to install
2. **Download Binary**: Attempts to download pre-built binary from releases
3. **Cache**: Stores binary in GitHub Actions cache
4. **Fallback**: If download fails, builds from source automatically
5. **PATH Update**: Adds syspac to PATH for easy access

## Caching

The action automatically caches downloaded binaries using GitHub Actions cache:

- **Cache Key**: `syspac-{version}-{os}`
- **Cache Location**: `~/.local/bin/syspac`
- **Cache Duration**: Follows GitHub's cache retention policy (usually 7 days)

### Manual Cache Control

```yaml
# Disable caching by building from source
- uses: hazayan/syspac@main
  with:
    version: build

# Clear cache by changing version
- uses: hazayan/syspac@main
  with:
    version: v0.2.1  # New version = new cache key
```

## Troubleshooting

### Binary Not Found

**Problem**: "syspac: command not found"

**Solution**: The action adds syspac to PATH, but you need to run it after the setup step:

```yaml
- uses: hazayan/syspac@main  # Must come first
- run: syspac detect-changes  # Now available
```

### Download Fails

**Problem**: "Failed to download syspac"

**Solution**: Action automatically falls back to building from source. If you want to force source build:

```yaml
- uses: hazayan/syspac@main
  with:
    version: build
```

### Permission Denied

**Problem**: "Permission denied" when downloading

**Solution**: Provide a GitHub token with appropriate permissions:

```yaml
- uses: hazayan/syspac@main
  with:
    github-token: ${{ secrets.GITHUB_TOKEN }}
```

### Version Not Found

**Problem**: "Release not found"

**Solution**: Check available versions:

```bash
gh release list --repo hazayan/syspac
```

Or use `latest`:

```yaml
- uses: hazayan/syspac@main
  with:
    version: latest
```

## Binary Releases

Pre-built binaries are available for:

- **Linux x86_64 (glibc)**: Most common, recommended
- **Linux x86_64 (musl)**: Static binary, no dependencies

### Binary Naming

- Standard: `syspac-linux-x86_64`
- Static: `syspac-linux-x86_64-musl`

The action automatically selects the appropriate binary for your platform.

## Building and Releasing Binaries

To create a new release with pre-built binaries:

### Automatic (on code push)

Binaries are automatically built and released when you push changes to `src/` or `Cargo.toml`:

```bash
git add src/
git commit -m "feat: add new feature"
git push
```

### Manual Release

Trigger a manual release with a specific version:

1. Go to **Actions** → **Release Syspac Tool**
2. Click **Run workflow**
3. Enter version (e.g., `v0.2.0`)
4. Click **Run workflow**

### Via Command Line

```bash
gh workflow run release-syspac.yml -f version=v0.2.0
```

## Advanced Usage

### Custom Installation Location

```yaml
- name: Setup Syspac
  uses: hazayan/syspac@main
  id: syspac

- name: Copy to custom location
  run: |
    mkdir -p /opt/tools
    cp ${{ steps.syspac.outputs.syspac-path }} /opt/tools/
```

### Multiple Versions

```yaml
- name: Setup Latest
  uses: hazayan/syspac@main
  id: latest

- name: Setup Specific
  uses: hazayan/syspac@main
  with:
    version: v0.1.0
  id: specific

- name: Compare versions
  run: |
    echo "Latest: ${{ steps.latest.outputs.syspac-version }}"
    echo "Specific: ${{ steps.specific.outputs.syspac-version }}"
```

### Conditional Setup

```yaml
- name: Setup Syspac (only on main branch)
  if: github.ref == 'refs/heads/main'
  uses: hazayan/syspac@main
```

## Related Documentation

- [README.md](../README.md) - General usage
- [QUICKSTART.md](../QUICKSTART.md) - Getting started
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Technical details
- [Release Workflow](.github/workflows/release-syspac.yml) - Binary build process

## Contributing

To improve the action:

1. Edit `action.yml`
2. Test locally with [act](https://github.com/nektos/act)
3. Submit a pull request

## License

See [LICENSE](../LICENSE) for details.
