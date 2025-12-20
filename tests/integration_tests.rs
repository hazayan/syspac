use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a test git repository
fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(path)
        .output()
        .unwrap();

    // Configure git
    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .unwrap();

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .unwrap();

    dir
}

/// Helper to create a simple PKGBUILD file
fn create_pkgbuild(dir: &Path, pkgver: &str, pkgrel: &str) {
    let content = format!(
        r#"# Maintainer: Test <test@example.com>
pkgname=test-package
pkgver={}
pkgrel={}
pkgdesc="Test package"
arch=('x86_64')
license=('MIT')

package() {{
    echo "test"
}}
"#,
        pkgver, pkgrel
    );

    fs::write(dir.join("PKGBUILD"), content).unwrap();
}

#[test]
fn test_list_packages_empty_repo() {
    let repo = create_test_repo();

    // Create initial commit
    fs::write(repo.path().join("README.md"), "# Test").unwrap();
    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Build syspac (assumes it's built)
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "list-packages",
            "-r",
            repo.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "");
}

#[test]
fn test_list_packages_with_package() {
    let repo = create_test_repo();

    // Create a package directory
    let pkg_dir = repo.path().join("packages").join("test-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();
    create_pkgbuild(&pkg_dir, "1.0.0", "1");

    // Commit
    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(&["commit", "-m", "Add test package"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // List packages
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "list-packages",
            "-r",
            repo.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test-pkg"));
}

#[test]
fn test_detect_changes_first_commit() {
    let repo = create_test_repo();

    // Create a package
    let pkg_dir = repo.path().join("test-pkg");
    fs::create_dir(&pkg_dir).unwrap();
    create_pkgbuild(&pkg_dir, "1.0.0", "1");

    // Commit
    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(&["commit", "-m", "First commit"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Detect changes (should return all packages on first commit)
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "detect-changes",
            "-r",
            repo.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("test-pkg"));
}

#[test]
fn test_package_version() {
    let dir = TempDir::new().unwrap();
    create_pkgbuild(dir.path(), "2.5.1", "3");

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "package-version",
            dir.path().join("PKGBUILD").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "2.5.1-3");
}

#[test]
fn test_detect_changes_json_format() {
    let repo = create_test_repo();

    // Create initial package
    let pkg_dir = repo.path().join("pkg1");
    fs::create_dir(&pkg_dir).unwrap();
    create_pkgbuild(&pkg_dir, "1.0.0", "1");

    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(&["commit", "-m", "Add pkg1"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Test JSON output format
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "detect-changes",
            "-r",
            repo.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok());
}

#[test]
fn test_removed_package_not_reported_as_changed() {
    let repo = create_test_repo();

    // Create a package under packages/
    let pkg_dir = repo.path().join("packages").join("to-remove");
    fs::create_dir_all(&pkg_dir).unwrap();
    create_pkgbuild(&pkg_dir, "1.0.0", "1");

    // Commit with the package present
    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(&["commit", "-m", "Add removable package"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // Record this commit as the base ref
    let base_output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(base_output.status.success());
    let base_ref = String::from_utf8(base_output.stdout).unwrap();
    let base_ref = base_ref.trim();

    // Now remove the package directory and commit the removal
    fs::remove_dir_all(&pkg_dir).unwrap();
    Command::new("git")
        .args(&["add", "-A"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(&["commit", "-m", "Remove package"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // list-packages at HEAD should *not* include the removed package
    let list_output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "list-packages",
            "-r",
            repo.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let list_stdout = String::from_utf8(list_output.stdout).unwrap();
    assert!(
        !list_stdout.contains("to-remove"),
        "removed package should not appear in list-packages output"
    );

    // detect-changes from the recorded base_ref should *not* report the removed package,
    // documenting the current limitation that deletions are not surfaced as 'changed'
    let detect_output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "detect-changes",
            "-r",
            repo.path().to_str().unwrap(),
            "--base-ref",
            base_ref,
        ])
        .output()
        .unwrap();
    assert!(detect_output.status.success());
    let detect_stdout = String::from_utf8(detect_output.stdout).unwrap();
    assert!(
        !detect_stdout.contains("to-remove"),
        "removed package should not be reported as changed by detect-changes according to current semantics"
    );
}
