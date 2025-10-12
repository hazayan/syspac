use anyhow::{Context, Result};
use git2::Repository;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a package in the repository
#[derive(Debug, Clone)]
pub struct Package {
    /// Package name (directory name)
    pub name: String,
    /// Relative path from repo root
    pub path: String,
    /// Full path to PKGBUILD
    pub pkgbuild_path: String,
    /// Whether this is a git submodule
    pub is_submodule: bool,
}

/// Finds all packages in the repository
/// This includes both git submodules and direct directories with PKGBUILD
pub fn find_all_packages(repo_path: &str) -> Result<Vec<Package>> {
    let repo = Repository::open(repo_path)
        .context(format!("Failed to open repository at {}", repo_path))?;

    let mut packages = Vec::new();
    let repo_path_buf = PathBuf::from(repo_path);

    // Find packages from submodules
    packages.extend(find_submodule_packages(&repo, &repo_path_buf)?);

    // Find direct directory packages (non-submodules)
    packages.extend(find_direct_packages(&repo_path_buf)?);

    // Sort by name for consistent output
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(packages)
}

/// Finds packages that are git submodules with PKGBUILD
fn find_submodule_packages(repo: &Repository, repo_path: &Path) -> Result<Vec<Package>> {
    let mut packages = Vec::new();

    // Get submodules
    let submodules = repo.submodules().context("Failed to get submodules")?;

    for submodule in submodules {
        let submodule_path = submodule.path();
        let full_path = repo_path.join(submodule_path);
        let pkgbuild_path = full_path.join("PKGBUILD");

        if pkgbuild_path.exists() {
            let name = submodule
                .name()
                .unwrap_or_else(|| {
                    submodule_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                })
                .to_string();

            packages.push(Package {
                name,
                path: submodule_path.to_string_lossy().to_string(),
                pkgbuild_path: pkgbuild_path.to_string_lossy().to_string(),
                is_submodule: true,
            });
        }
    }

    Ok(packages)
}

/// Finds packages in direct directories (not submodules)
/// Searches up to 2 levels deep for PKGBUILD files
fn find_direct_packages(repo_path: &Path) -> Result<Vec<Package>> {
    let mut packages = Vec::new();

    // Search for PKGBUILD files up to 2 levels deep
    for entry in fs::read_dir(repo_path).context("Failed to read repository directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // Skip if it's a git submodule (has .git directory/file)
        if is_submodule_dir(&path) {
            continue;
        }

        // Skip hidden directories and common non-package directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.')
                || name == "target"
                || name == "node_modules"
                || name == "build-container"
                || name == "repo"
            {
                continue;
            }
        }

        // Check if this directory has a PKGBUILD
        if path.is_dir() {
            let pkgbuild_path = path.join("PKGBUILD");
            if pkgbuild_path.exists() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let rel_path = path
                    .strip_prefix(repo_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                packages.push(Package {
                    name,
                    path: rel_path,
                    pkgbuild_path: pkgbuild_path.to_string_lossy().to_string(),
                    is_submodule: false,
                });
                continue;
            }

            // Check one level deeper
            if let Ok(entries) = fs::read_dir(&path) {
                for sub_entry in entries {
                    if let Ok(sub_entry) = sub_entry {
                        let sub_path = sub_entry.path();

                        if sub_path.is_dir() && !is_submodule_dir(&sub_path) {
                            let pkgbuild_path = sub_path.join("PKGBUILD");
                            if pkgbuild_path.exists() {
                                let name = sub_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                let rel_path = sub_path
                                    .strip_prefix(repo_path)
                                    .unwrap_or(&sub_path)
                                    .to_string_lossy()
                                    .to_string();

                                packages.push(Package {
                                    name,
                                    path: rel_path,
                                    pkgbuild_path: pkgbuild_path.to_string_lossy().to_string(),
                                    is_submodule: false,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(packages)
}

/// Checks if a directory is a git submodule
fn is_submodule_dir(path: &Path) -> bool {
    // A submodule has either a .git file (pointing to parent repo) or .git directory
    path.join(".git").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_submodule_dir() {
        // This would need proper test fixtures
        assert!(!is_submodule_dir(Path::new("/nonexistent")));
    }

    #[test]
    fn test_find_all_packages_invalid_repo() {
        let result = find_all_packages("/nonexistent/path");
        assert!(result.is_err());
    }
}
