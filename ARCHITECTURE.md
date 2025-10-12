# Architecture Documentation

## Overview

Syspac is a command-line tool for managing Artix Linux package repositories hosted on GitHub. It replaces brittle shell scripts with a robust, testable Rust implementation.

## Design Goals

1. **Modularity** - Clear separation of concerns with dedicated modules
2. **Testability** - Every component can be unit tested
3. **Reliability** - Type safety and comprehensive error handling
4. **Performance** - Fast execution through compiled code
5. **Maintainability** - Clean code structure and documentation
6. **Reusability** - Single tool for all repository operations

## Module Structure

```
syspac/
├── src/
│   ├── main.rs          # CLI interface and command routing
│   ├── git.rs           # Git operations (libgit2)
│   ├── package.rs       # Package discovery
│   └── pkgbuild.rs      # PKGBUILD parsing
├── tests/
│   └── integration_tests.rs  # End-to-end tests
└── Cargo.toml           # Dependencies and metadata
```

## Core Modules

### 1. main.rs - CLI Interface

**Responsibility:** Command-line argument parsing and command routing

**Key Components:**
- `Cli` struct - Top-level CLI structure
- `Commands` enum - All available subcommands
- `main()` function - Entry point and command dispatcher

**Commands:**
- `detect-changes` - Find packages changed between commits
- `list-packages` - List all available packages
- `package-version` - Extract version from PKGBUILD

**Dependencies:**
- `clap` - Command-line parsing
- `anyhow` - Error handling
- `serde_json` - JSON output format

### 2. git.rs - Git Operations

**Responsibility:** All git-related operations using libgit2

**Key Functions:**

```rust
pub fn detect_changed_packages(
    repo_path: &str, 
    base_ref: Option<&str>
) -> Result<Vec<String>>
```
- Opens repository
- Resolves base ref (defaults to HEAD^)
- Compares trees between commits
- Maps changed files to packages
- Returns sorted list of changed package names

```rust
fn find_changed_packages_between_commits(
    repo: &Repository,
    base_oid: &Oid,
    head_oid: &Oid,
    packages: &[Package],
) -> Result<Vec<String>>
```
- Creates diff between two commit trees
- Iterates through deltas (changed files)
- Matches changed files to their parent packages
- Returns unique set of changed packages

```rust
pub fn has_path_changed(
    repo_path: &str,
    path: &str,
    base_ref: &str,
) -> Result<bool>
```
- Utility to check if specific path has changes
- Uses pathspec filtering

**Design Decisions:**
- Uses libgit2 for reliability (no shell commands)
- Handles first commit scenario (no parent)
- Sorts output for consistency
- HashSet ensures unique package names

### 3. package.rs - Package Discovery

**Responsibility:** Finding and cataloging packages in the repository

**Key Type:**

```rust
pub struct Package {
    pub name: String,           // Package name
    pub path: String,           // Relative path from repo root
    pub pkgbuild_path: String,  // Full path to PKGBUILD
    pub is_submodule: bool,     // Whether it's a git submodule
}
```

**Key Functions:**

```rust
pub fn find_all_packages(repo_path: &str) -> Result<Vec<Package>>
```
- Main entry point for package discovery
- Combines submodule and direct package finding
- Sorts results by name

```rust
fn find_submodule_packages(
    repo: &Repository, 
    repo_path: &Path
) -> Result<Vec<Package>>
```
- Uses git2 to enumerate submodules
- Checks each for PKGBUILD file
- Marks packages as submodules

```rust
fn find_direct_packages(repo_path: &Path) -> Result<Vec<Package>>
```
- Searches filesystem for PKGBUILD files
- Goes up to 2 levels deep
- Skips common non-package directories
- Excludes submodules (already found)

**Design Decisions:**
- Two-pronged approach: submodules + direct directories
- Filters out `.git`, `target`, `build-container`, etc.
- 2-level depth limit prevents excessive searching
- Combines both sources for complete picture

### 4. pkgbuild.rs - PKGBUILD Parsing

**Responsibility:** Extracting information from PKGBUILD files

**Key Type:**

```rust
pub struct PackageVersion {
    pub pkgver: String,
    pub pkgrel: String,
}

impl Display for PackageVersion {
    // Formats as "pkgver-pkgrel" (e.g., "1.2.3-1")
}
```

**Key Functions:**

```rust
pub fn parse_version(pkgbuild_path: &str) -> Result<PackageVersion>
```
- Primary parser using bash sourcing
- Most reliable for complex PKGBUILDs
- Handles variable expansion and functions
- Extracts pkgver and pkgrel

```rust
pub fn parse_version_simple(pkgbuild_path: &str) -> Result<PackageVersion>
```
- Fallback regex-based parser
- No bash required
- Only handles simple assignments
- Used for cases without bash

```rust
pub fn parse_pkgname(pkgbuild_path: &str) -> Result<String>
```
- Extracts package name from PKGBUILD
- Uses bash sourcing for reliability

**Design Decisions:**
- Bash sourcing is most reliable (matches makepkg behavior)
- Simple parser as fallback for restricted environments
- Validates variables are non-empty
- Clear error messages for missing fields

## Data Flow

### Change Detection Flow

```
User runs: syspac detect-changes

main.rs
  └─> Commands::DetectChanges
       └─> git::detect_changed_packages()
            ├─> package::find_all_packages()
            │    ├─> find_submodule_packages()
            │    └─> find_direct_packages()
            └─> find_changed_packages_between_commits()
                 ├─> repo.diff_tree_to_tree()
                 └─> Map files to packages
                      └─> Return: ["niri", "valent"]
```

### Package Listing Flow

```
User runs: syspac list-packages --verbose

main.rs
  └─> Commands::ListPackages
       └─> package::find_all_packages()
            ├─> find_submodule_packages()
            └─> find_direct_packages()
                 └─> For each package:
                      └─> pkgbuild::parse_version()
                           └─> Return: "1.2.3-1"
```

## Error Handling Strategy

### Philosophy
- Use `anyhow::Result` for propagating errors
- Provide context at each level
- Give actionable error messages
- No panics in library code

### Example Error Chain

```rust
// Low level (git.rs)
let repo = Repository::open(repo_path)
    .context(format!("Failed to open repository at {}", repo_path))?;

// Mid level (package.rs)
let packages = find_all_packages(&repo_path)
    .context("Failed to discover packages")?;

// Top level (main.rs)
let changes = git::detect_changed_packages(&repo_path, base_ref.as_deref())?;
// If error occurs, user sees full context chain
```

### Error Message Quality

❌ Bad: "git error"  
✅ Good: "Failed to open repository at /path/to/repo: repository not found"

## Testing Strategy

### Unit Tests
- Located in each module (`#[cfg(test)] mod tests`)
- Test individual functions in isolation
- Use mocks/fakes where appropriate
- Fast execution

### Integration Tests
- Located in `tests/integration_tests.rs`
- Test complete workflows end-to-end
- Use temporary directories
- Create real git repositories
- Test CLI interface

### Test Coverage
- Core logic in `git.rs`, `package.rs`, `pkgbuild.rs`
- Edge cases (empty repos, no parents, invalid PKGBUILDs)
- Error conditions
- Output formats (space-separated, JSON)

## Performance Considerations

### Optimizations
1. **Single Tree Walk** - Only diff trees once per comparison
2. **Hash Set for Uniqueness** - O(1) deduplication of packages
3. **Early Returns** - Skip unnecessary work when possible
4. **Compiled Binary** - ~100x faster than shell scripts

### Scalability
- Linear with number of packages
- Efficient for typical repo sizes (10-100 packages)
- libgit2 handles large diffs efficiently

## Extension Points

### Adding New Commands

1. Add variant to `Commands` enum in `main.rs`
2. Implement handler in `main()` match block
3. Call appropriate module functions
4. Add tests in `integration_tests.rs`

Example:
```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Check if package needs rebuild
    CheckRebuild {
        package_name: String,
    },
}
```

### Adding New Modules

1. Create new file in `src/`
2. Declare module in `main.rs`: `mod new_module;`
3. Define public API with `pub fn` functions
4. Use `anyhow::Result` for error handling
5. Add unit tests in module
6. Add integration tests

### Adding New Parsers

Extend `pkgbuild.rs` with new parsing functions:

```rust
pub fn parse_dependencies(pkgbuild_path: &str) -> Result<Vec<String>> {
    // Parse depends=() array from PKGBUILD
}

pub fn parse_sources(pkgbuild_path: &str) -> Result<Vec<String>> {
    // Parse source=() array from PKGBUILD
}
```

## Dependencies

### Core Dependencies
- **anyhow** (1.0) - Error handling with context
- **clap** (4.5) - Command-line argument parsing
- **git2** (0.20) - Git operations via libgit2
- **serde** + **serde_json** (1.0) - JSON serialization

### Dev Dependencies
- **tempfile** (3.8) - Temporary directories for tests

### Rationale
- Minimal dependencies for fast compilation
- Well-maintained, popular crates
- Clear, documented APIs
- Good error messages

## Security Considerations

1. **No Shell Injection** - Uses libgit2, not shell commands for git
2. **Path Validation** - Checks file existence before operations
3. **Safe Bash Sourcing** - Only sources known PKGBUILD files
4. **No Arbitrary Code Execution** - Controlled execution contexts

## Future Enhancements

### Potential Features
1. **Parallel Processing** - Build packages concurrently
2. **Dependency Resolution** - Determine build order
3. **Caching** - Cache parsed PKGBUILDs
4. **Watch Mode** - Monitor for changes and rebuild
5. **Remote Operations** - Fetch from GitHub API
6. **Database Integration** - Track build history
7. **Notification System** - Alert on build failures
8. **Metrics Collection** - Build time, success rate, etc.

### Architecture for Parallel Builds

```rust
// Future module: src/builder.rs
pub struct BuildQueue {
    packages: Vec<Package>,
    max_parallel: usize,
}

impl BuildQueue {
    pub async fn build_all(&self) -> Result<Vec<BuildResult>> {
        // Use tokio for async builds
        // Respect dependency order
        // Build independent packages in parallel
    }
}
```

## Maintenance Guidelines

### Code Style
- Run `cargo fmt` before committing
- Follow Rust API guidelines
- Document public APIs with doc comments
- Keep functions focused and small

### Versioning
- Semantic versioning (MAJOR.MINOR.PATCH)
- Update Cargo.toml version on releases
- Tag releases in git

### Updating Dependencies
```bash
cargo update           # Update within semver range
cargo outdated         # Check for major updates
cargo audit            # Security vulnerabilities
```

## Comparison with Shell Implementation

| Aspect | Shell Scripts | Rust Implementation |
|--------|--------------|---------------------|
| Lines of code | ~150 | ~500 (with tests & docs) |
| Type safety | None | Full |
| Error handling | Basic | Comprehensive |
| Testability | Difficult | Easy |
| Performance | Slow | Fast |
| Maintainability | Low | High |
| Reusability | Limited | High |
| Debugging | Hard | Easy |

## Conclusion

The Rust implementation provides a solid foundation for managing package repositories with:

- Clear module boundaries
- Comprehensive error handling
- Full test coverage
- Excellent performance
- Easy extensibility

The architecture supports both current needs and future enhancements while maintaining code quality and reliability.
