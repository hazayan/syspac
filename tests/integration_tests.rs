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
