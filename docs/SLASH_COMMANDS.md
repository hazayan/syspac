# Slash Commands

This document describes the slash commands available in the repository for triggering builds and other actions.

## Available Commands

### `/rebuild-all`

Triggers a full rebuild of all packages in the repository, regardless of whether they've changed.

**Usage:**

1. **On Pull Requests**: Comment `/rebuild-all` on any PR
2. **On Issues**: Comment `/rebuild-all` on any issue (maintainers only)

**Who can use it:**
- Repository owners
- Repository members
- Collaborators (on PRs)

**What it does:**
1. Acknowledges the command with a 🚀 reaction
2. Lists all packages that will be rebuilt
3. Triggers the build workflow with the `--all` flag
4. All packages will be built and the repository release will be updated

**Example:**

```
/rebuild-all
```

The bot will respond with:

```
🔨 **Rebuild All Packages Triggered**

Rebuilding all packages: `niri valent ly connman-resolvd`

Watch the progress: [Build Workflow](...)
```

## How It Works

### Architecture

```
PR/Issue Comment (/rebuild-all)
         ↓
.github/workflows/slash-commands.yml
         ↓
  [Validates permissions]
         ↓
  [Adds reaction 🚀]
         ↓
  [Triggers repository_dispatch]
         ↓
.github/workflows/build-rust.yml
         ↓
  [Detects event type]
         ↓
  syspac detect-changes --all
         ↓
  [Builds all packages]
         ↓
  [Updates release with all packages]
```

### Technical Details

1. **Slash Commands Workflow** (`.github/workflows/slash-commands.yml`):
   - Listens for `issue_comment` events
   - Checks if comment contains `/rebuild-all`
   - Validates user permissions
   - Triggers `repository_dispatch` event with type `rebuild-all`

2. **Build Workflow** (`.github/workflows/build-rust.yml`):
   - Listens for both `push` and `repository_dispatch` events
   - When `repository_dispatch` type is `rebuild-all`, uses `syspac detect-changes --all`
   - Otherwise uses normal change detection

3. **Syspac Tool**:
   - `syspac detect-changes` - Returns only changed packages
   - `syspac detect-changes --all` - Returns all packages in repository

## Use Cases

### When to use `/rebuild-all`

1. **Dependency updates**: When a core dependency changes that affects all packages
2. **Toolchain updates**: After updating Rust compiler, build tools, etc.
3. **Repository corruption**: If the release is corrupted or packages are missing
4. **Testing**: To verify all packages still build correctly
5. **Major changes**: After significant changes to build infrastructure

### When NOT to use `/rebuild-all`

1. **Normal PR changes**: The workflow automatically detects changed packages
2. **Single package updates**: Just commit the change, it will build automatically
3. **Documentation changes**: These don't affect package builds

## Permissions

### Permission Levels

| Association | Can use /rebuild-all on PRs | Can use /rebuild-all on Issues |
|-------------|----------------------------|-------------------------------|
| OWNER       | ✅ Yes                      | ✅ Yes                         |
| MEMBER      | ✅ Yes                      | ✅ Yes                         |
| COLLABORATOR| ✅ Yes                      | ❌ No                          |
| CONTRIBUTOR | ❌ No                       | ❌ No                          |

### Security Considerations

- Rebuilds are rate-limited by GitHub Actions quotas
- Only trusted users can trigger rebuilds to prevent abuse
- All rebuild actions are logged in GitHub Actions
- Failed rebuilds do not remove existing packages

## Monitoring

### Check Rebuild Status

1. **GitHub Actions Tab**: View all workflow runs
2. **Comment Response**: Bot provides direct link to workflow run
3. **Release Page**: Check updated timestamp and package list

### Logs

All rebuild actions are logged:
- Who triggered the rebuild
- When it was triggered
- Which packages were built
- Build success/failure status

## Troubleshooting

### Command not recognized

**Problem**: Bot doesn't respond to `/rebuild-all`

**Solutions**:
- Ensure you have the correct permissions
- Check that slash-commands.yml workflow is enabled
- Verify you're commenting on a PR or issue (not a commit)
- Make sure the comment contains exactly `/rebuild-all` (case-sensitive)

### Rebuild fails

**Problem**: The rebuild workflow fails

**Solutions**:
- Check the workflow logs in GitHub Actions
- Verify all packages have valid PKGBUILDs
- Check for build dependencies
- Look for error messages in the build logs

### Packages missing after rebuild

**Problem**: Some packages disappeared after rebuild

**Solutions**:
- Check the workflow logs for build failures
- Verify the package directories still exist
- Check if PKGBUILD files are valid
- Review the "Download existing release assets" step

## Adding New Slash Commands

Want to add more slash commands? Here's how:

### 1. Add to slash-commands.yml

```yaml
my-command:
  if: |
    github.event.issue.pull_request &&
    contains(github.event.comment.body, '/my-command')
  runs-on: ubuntu-latest
  steps:
    - name: Do something
      run: echo "My command"
```

### 2. Update this documentation

Document the new command, its permissions, and use cases.

### 3. Test thoroughly

Test on a PR before merging to main.

## Examples

### Example 1: Rebuild all packages on a PR

```markdown
The build system was updated, let's rebuild everything to ensure compatibility.

/rebuild-all
```

### Example 2: Force rebuild after infrastructure change

```markdown
After updating the build container, we should rebuild all packages:

/rebuild-all
```

### Example 3: Testing before merge

```markdown
Before merging this PR, let's verify all packages still build:

/rebuild-all
```

## FAQ

**Q: How long does `/rebuild-all` take?**
A: Depends on the number of packages and their complexity. Typically 10-30 minutes for small repositories.

**Q: Does `/rebuild-all` replace existing packages?**
A: Yes, it rebuilds packages from source, but preserves packages that aren't rebuilt.

**Q: Can I cancel a `/rebuild-all`?**
A: Yes, go to the GitHub Actions tab and cancel the workflow run.

**Q: Does `/rebuild-all` work on forks?**
A: Yes, if the workflows are enabled in the fork.

**Q: What happens if a package fails to build during `/rebuild-all`?**
A: The workflow will fail and show which package caused the failure. Existing packages remain untouched.

## Related Documentation

- [README.md](../README.md) - General usage
- [MIGRATION.md](../MIGRATION.md) - Migration from shell scripts
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Technical architecture
