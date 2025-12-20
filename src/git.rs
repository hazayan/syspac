use anyhow::{Context, Result};
use git2::{DiffOptions, Oid, Repository};
use std::collections::HashSet;

use crate::package::{find_all_packages, Package};

/// Detects packages that have changed between the base ref and HEAD
pub fn detect_changed_packages(repo_path: &str, base_ref: Option<&str>) -> Result<Vec<String>> {
    let repo = Repository::open(repo_path)
        .context(format!("Failed to open repository at {}", repo_path))?;

    // Get all packages first
    let all_packages = find_all_packages(repo_path)?;

    // If no base ref provided, try to get HEAD^ (parent of current commit)
    let base_ref = match base_ref {
        Some(r) => r.to_string(),
        None => {
            // Try to get HEAD^
            match get_head_parent(&repo) {
                Ok(oid) => oid.to_string(),
                Err(_) => {
                    // First commit or no parent available - return all packages
                    return Ok(all_packages.iter().map(|p| p.name.clone()).collect());
                }
            }
        }
    };

    // Parse the base reference
    let base_object = repo
        .revparse_single(&base_ref)
        .context(format!("Failed to parse base ref: {}", base_ref))?;
    let base_commit = base_object
        .peel_to_commit()
        .context("Failed to peel base ref to commit")?;

    // Get HEAD commit
    let head = repo.head().context("Failed to get HEAD")?;
    let head_commit = head
        .peel_to_commit()
        .context("Failed to peel HEAD to commit")?;

    // Find changed packages
    let changed = find_changed_packages_between_commits(
        &repo,
        &base_commit.id(),
        &head_commit.id(),
        &all_packages,
    )?;

    Ok(changed)
}

/// Gets the parent commit of HEAD
fn get_head_parent(repo: &Repository) -> Result<Oid> {
    let head = repo.head().context("Failed to get HEAD")?;
    let head_commit = head
        .peel_to_commit()
        .context("Failed to peel HEAD to commit")?;

    if head_commit.parent_count() == 0 {
        anyhow::bail!("HEAD has no parent (first commit)");
    }

    Ok(head_commit.parent_id(0)?)
}

/// Finds packages that have changed between two commits
fn find_changed_packages_between_commits(
    repo: &Repository,
    base_oid: &Oid,
    head_oid: &Oid,
    packages: &[Package],
) -> Result<Vec<String>> {
    let base_commit = repo.find_commit(*base_oid)?;
    let head_commit = repo.find_commit(*head_oid)?;

    let base_tree = base_commit.tree()?;
    let head_tree = head_commit.tree()?;

    let mut changed_packages = HashSet::new();

    // Create diff between the two trees
    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;

    // Check each delta (changed file) to see which package it belongs to
    for delta in diff.deltas() {
        // Consider both old and new paths so we correctly detect renames and deletions
        let mut candidate_paths = Vec::new();

        if let Some(path) = delta.new_file().path() {
            candidate_paths.push(path.to_string_lossy().to_string());
        }

        if let Some(path) = delta.old_file().path() {
            candidate_paths.push(path.to_string_lossy().to_string());
        }

        for path_str in candidate_paths {
            // Check if this path belongs to any package
            for package in packages {
                if path_str.starts_with(&package.path) {
                    changed_packages.insert(package.name.clone());
                    break;
                }
            }
        }
    }

    let mut result: Vec<String> = changed_packages.into_iter().collect();
    result.sort();
    Ok(result)
}

/// Checks if a path has changes between two commits
pub fn has_path_changed(repo_path: &str, path: &str, base_ref: &str) -> Result<bool> {
    let repo = Repository::open(repo_path)?;

    let base_object = repo.revparse_single(base_ref)?;
    let base_commit = base_object.peel_to_commit()?;

    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;

    let base_tree = base_commit.tree()?;
    let head_tree = head_commit.tree()?;

    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(path);

    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut diff_opts))?;

    Ok(diff.deltas().len() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_changes_invalid_repo() {
        let result = detect_changed_packages("/nonexistent/path", None);
        assert!(result.is_err());
    }

    // Additional tests would require setting up test git repositories
    // Consider using tempdir and git2 to create test fixtures
}
