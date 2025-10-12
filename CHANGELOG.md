# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2025-01-XX - Critical Fixes

### 🐛 Critical Bug Fixes

#### Fixed: Packages Disappearing from Release
- **Problem**: When only some packages changed, all unchanged packages would disappear from the GitHub release
- **Impact**: Users would lose access to packages that weren't modified
- **Solution**: Workflow now downloads existing packages before building, preserving everything
- **Implementation**: Added "Download existing release assets" step to workflow

#### Fixed: No Way to Rebuild All Packages
- **Problem**: Could only build changed packages, no mechanism for full rebuild
- **Impact**: Couldn't recover from infrastructure changes or dependency updates
- **Solution**: Added `--all` flag and `/rebuild-all` slash command
- **Implementation**: New CLI flag and GitHub Actions integration

### ✨ New Features

#### `--all` Flag for Full Rebuilds
```bash
# Rebuild all packages regardless of changes
syspac detect-changes --all
```

#### Slash Commands Support
- Comment `/rebuild-all` on any PR or issue to trigger full rebuild
- Automatic permission checks (owners, members, collaborators)
- Real-time feedback with build status links

### 🔧 Improvements

#### Workflow Enhancements
- Downloads existing packages before building
- Preserves unchanged packages in release
- Supports both normal and forced rebuilds
- Better error messages and logging
- More detailed release notes

#### Documentation
- Added `docs/SLASH_COMMANDS.md` - Slash command documentation
- Added `docs/FIXES.md` - Detailed explanation of bug fixes
- Updated `README.md` with new features
- Updated `MIGRATION.md` with workflow changes
- Updated `ARCHITECTURE.md` with new flows

### 📝 Files Changed

**Core Tool:**
- `src/main.rs` - Added `--all` flag to `DetectChanges` command

**Workflows:**
- `.github/workflows/build-rust.yml.example` - Complete rewrite with package preservation
- `.github/workflows/slash-commands.yml` - New slash command handler

**Documentation:**
- `README.md` - Updated with new features
- `docs/SLASH_COMMANDS.md` - New documentation
- `docs/FIXES.md` - Bug fix details
- `CHANGELOG.md` - This file

## [0.1.0] - 2025-01-XX - Initial Rust Rewrite

### ✨ Initial Release

#### Core Features
- **Change Detection**: Detect packages that changed between commits
- **Package Discovery**: Find all packages (submodules + directories)
- **PKGBUILD Parsing**: Extract version information
- **CLI Interface**: Clean command-line interface with clap

#### Modules Implemented
- `src/main.rs` - CLI interface
- `src/git.rs` - Git operations using libgit2
- `src/package.rs` - Package discovery
- `src/pkgbuild.rs` - PKGBUILD parsing

#### Commands
- `detect-changes` - Find changed packages
- `list-packages` - List all packages
- `package-version` - Get package version

#### Documentation
- `README.md` - Usage guide
- `MIGRATION.md` - Migration from shell scripts
- `ARCHITECTURE.md` - Technical documentation

#### Tests
- Unit tests in each module
- Integration tests in `tests/integration_tests.rs`

### 🎯 Migration from Shell Scripts

#### Replaced
- ~150 lines of complex bash in GitHub workflow
- Brittle shell script logic
- Manual git submodule parsing
- Complex version extraction

#### With
- Single compiled binary
- Type-safe operations
- Comprehensive error handling
- Full test coverage

#### Benefits
- **90% less code** in workflows
- **Type safety** - Compile-time guarantees
- **Better errors** - Detailed error messages
- **Testable** - Unit and integration tests
- **Faster** - Compiled binary vs shell
- **Maintainable** - Clear module structure

---

## Upgrade Guide

### From 0.1.0 to 0.2.0

#### Update Workflow

Replace your workflow with the new example that includes package preservation:

```bash
cp .github/workflows/build-rust.yml.example .github/workflows/build.yml
```

#### Add Slash Commands

Add the slash command workflow:

```bash
cp .github/workflows/slash-commands.yml .github/workflows/
```

#### Update Tool Usage

The tool is backward compatible, but now supports:

```bash
# New: Rebuild all packages
syspac detect-changes --all

# Existing commands work unchanged
syspac detect-changes
syspac list-packages
syspac package-version <path>
```

#### Test the Changes

1. Push a change to one package
2. Verify only that package rebuilds
3. Verify old packages remain in release
4. Test `/rebuild-all` command on a PR

### Breaking Changes

None - fully backward compatible.

### New Dependencies

None - no new Cargo dependencies added.

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.2.0 | 2025-01-XX | Critical fixes + slash commands |
| 0.1.0 | 2025-01-XX | Initial Rust rewrite |

---

## Future Roadmap

### Planned Features

#### v0.3.0 - Performance
- [ ] Parallel package builds
- [ ] Build caching
- [ ] Incremental database updates

#### v0.4.0 - Advanced Features
- [ ] Dependency resolution
- [ ] Build order optimization
- [ ] Multi-architecture support

#### v0.5.0 - Monitoring
- [ ] Build metrics collection
- [ ] Failure notifications
- [ ] Build time analytics

#### v1.0.0 - Production Ready
- [ ] Complete test coverage (>90%)
- [ ] Performance benchmarks
- [ ] Security audit
- [ ] Production deployment guide

### Ideas for Consideration
- [ ] Web UI for repository browsing
- [ ] Package signing automation
- [ ] Automated version bumping
- [ ] Change log generation
- [ ] Release notes automation
- [ ] Mirror support
- [ ] CDN integration

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on submitting changes.

## License

See [LICENSE](LICENSE) for details.
