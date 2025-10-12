# Migration Guide: Shell Scripts → Rust

This guide explains how to migrate from the shell-based implementation to the new Rust-based tool.

## Overview of Changes

### Before: Shell-Based (.github/workflows/build.yml)
- ~100 lines of bash script for change detection
- Complex git submodule parsing
- Duplicated logic between workflow and Docker entrypoint
- Difficult to test
- Brittle error handling

### After: Rust-Based (syspac tool)
- Single `cargo run` command
- Type-safe operations
- Testable components
- Clear error messages
- Reusable across workflow and Docker

## Step-by-Step Migration

### Step 1: Test the Tool Locally

First, ensure the tool works with your repository:

```bash
# Build the tool
cargo build --release

# List all packages
./target/release/syspac list-packages --verbose

# Test change detection (will show all packages if this is first commit)
./target/release/syspac detect-changes

# Test with a specific base ref
./target/release/syspac detect-changes --base-ref HEAD~2
```

**Expected output:**
```
niri valent ly connman-resolvd
```

### Step 2: Update GitHub Workflow

Replace the complex bash logic in `.github/workflows/build.yml`:

#### Old Preflight Job (lines 14-81):
```yaml
- name: Detect changed packages
  id: changes
  run: |
    # ~60 lines of bash script
    get_base_ref() { ... }
    get_pkg_version() { ... }
    # ... complex logic ...
```

#### New Preflight Job:
```yaml
- name: Set up Rust
  uses: actions-rs/toolchain@v1
  with:
    profile: minimal
    toolchain: stable

- name: Build syspac tool
  run: cargo build --release

- name: Detect changed packages
  id: changes
  run: |
    CHANGED=$(./target/release/syspac detect-changes)
    echo "packages=${CHANGED}" >> $GITHUB_OUTPUT
    echo "Changed packages: ${CHANGED}"
```

**Benefits:**
- Reduced from ~60 lines to ~10 lines
- Faster execution (compiled binary)
- Better error messages
- Consistent behavior

### Step 3: Optional - Update Docker Entrypoint

You can also use the Rust tool inside your Docker build process:

#### Add to Dockerfile:
```dockerfile
# Install Rust (if not already present)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy and build syspac
COPY . /syspac
RUN cd /syspac && cargo build --release
```

#### Use in entrypoint.sh:
```bash
# Instead of complex bash logic to determine packages
if [ -n "${CHANGED_PACKAGES-}" ]; then
    echo "Processing packages: ${CHANGED_PACKAGES}"
    for pkg in ${CHANGED_PACKAGES}; do
        # ... build logic ...
    done
else
    # Optionally use syspac to detect changes
    CHANGED_PACKAGES=$(/syspac/target/release/syspac detect-changes)
    # ... continue ...
fi
```

### Step 4: Verify the Migration

Create a test branch and make some changes:

```bash
# Create test branch
git checkout -b test-rust-migration

# Modify a package
cd packages/niri
# ... make some changes to PKGBUILD or source ...
git add .
git commit -m "test: update niri"

# Test locally
cd ../..
./target/release/syspac detect-changes
# Should output: niri

# Push and check GitHub Actions
git push origin test-rust-migration
```

### Step 5: Clean Up Old Code

Once verified, you can:

1. Remove the complex bash functions from the workflow
2. Delete the old `.github/workflows/build.yml` 
3. Rename `.github/workflows/build-rust.yml.example` to `.github/workflows/build.yml`

## Comparison: Before vs After

### Change Detection

**Before (bash):**
```bash
get_base_ref() {
  git rev-parse HEAD^
}

get_pkg_version() {
  local pkgbuild="$1"
  if [ -f "$pkgbuild" ]; then
    (source "$pkgbuild" 2>/dev/null && echo "${pkgver}-${pkgrel}") || echo "unknown"
  else
    echo "unknown"
  fi
}

BASE_REF=$(get_base_ref) || BASE_REF=""
mapfile -t ALL_PKGS < <(
  git submodule foreach --quiet 'if [ -f "PKGBUILD" ]; then echo "$name"; fi'
  find . -maxdepth 2 -name PKGBUILD -exec dirname {} \; | while read -r dir; do
    dir=${dir#./}
    if [ -d "$dir" ] && [ ! -d "$dir/.git" ]; then
      echo "$dir"
    fi
  done | sort -u
)
# ... another 40 lines ...
```

**After (Rust):**
```bash
./target/release/syspac detect-changes
```

### Package Listing

**Before (bash):**
```bash
git submodule foreach --quiet 'if [ -f "PKGBUILD" ]; then echo "$name"; fi'
find . -maxdepth 2 -name PKGBUILD -exec dirname {} \; | while read -r dir; do
  dir=${dir#./}
  if [ -d "$dir" ] && [ ! -d "$dir/.git" ]; then
    echo "$dir"
  fi
done | sort -u
```

**After (Rust):**
```bash
./target/release/syspac list-packages
```

### Version Extraction

**Before (bash):**
```bash
get_pkg_version() {
  local pkgbuild="$1"
  if [ -f "$pkgbuild" ]; then
    (source "$pkgbuild" 2>/dev/null && echo "${pkgver}-${pkgrel}") || echo "unknown"
  else
    echo "unknown"
  fi
}
```

**After (Rust):**
```bash
./target/release/syspac package-version packages/niri
```

## Testing Checklist

Before deploying to production:

- [ ] Tool compiles successfully: `cargo build --release`
- [ ] All tests pass: `cargo test`
- [ ] Lists correct packages: `./target/release/syspac list-packages`
- [ ] Detects changes correctly: `./target/release/syspac detect-changes`
- [ ] JSON output works: `./target/release/syspac detect-changes --format json`
- [ ] Version parsing works: `./target/release/syspac package-version <path>`
- [ ] GitHub Actions workflow completes successfully
- [ ] Packages build correctly in Docker
- [ ] Repository updates as expected

## Rollback Plan

If issues arise, you can easily rollback:

1. Keep the old workflow file as `.github/workflows/build.yml.old`
2. Test the new implementation on a feature branch first
3. If needed, restore: `mv build.yml.old build.yml`

## Troubleshooting

### Tool doesn't detect changes

**Problem:** `syspac detect-changes` returns empty
**Solution:** 
- Check if you're on a branch with commits: `git log`
- Try specifying base ref: `syspac detect-changes --base-ref main`
- Verify packages exist: `syspac list-packages`

### PKGBUILD parsing fails

**Problem:** `package-version` command fails
**Solution:**
- Ensure bash is available: `which bash`
- Check PKGBUILD syntax: `bash -n path/to/PKGBUILD`
- Try sourcing manually: `source path/to/PKGBUILD && echo $pkgver-$pkgrel`

### Git operations fail

**Problem:** "Failed to open repository" error
**Solution:**
- Verify git repo: `git status`
- Check submodules: `git submodule status`
- Update submodules: `git submodule update --init --recursive`

## Additional Resources

- [README.md](README.md) - Full documentation
- [Integration Tests](tests/integration_tests.rs) - Test examples
- [Example Workflow](.github/workflows/build-rust.yml.example) - Complete workflow

## Support

If you encounter issues during migration:

1. Check the error message carefully (Rust provides detailed errors)
2. Run with `RUST_BACKTRACE=1` for stack traces
3. Review the integration tests for examples
4. File an issue with details of the problem

## Benefits Realized

After migration, you'll have:

✅ **Type Safety** - Compile-time guarantees  
✅ **Better Errors** - Clear, actionable error messages  
✅ **Testability** - Unit and integration tests  
✅ **Performance** - Faster execution  
✅ **Maintainability** - Clear module structure  
✅ **Reusability** - Use the same tool everywhere  
✅ **Reliability** - No more brittle shell scripts  

Happy migrating! 🦀
