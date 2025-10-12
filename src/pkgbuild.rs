use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Represents a parsed PKGBUILD version
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVersion {
    pub pkgver: String,
    pub pkgrel: String,
}

impl std::fmt::Display for PackageVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.pkgver, self.pkgrel)
    }
}

/// Parses version information from a PKGBUILD file
///
/// This uses bash to source the PKGBUILD and extract variables,
/// which is the most reliable way to handle complex PKGBUILDs
pub fn parse_version(pkgbuild_path: &str) -> Result<PackageVersion> {
    let path = Path::new(pkgbuild_path);

    if !path.exists() {
        anyhow::bail!("PKGBUILD not found at: {}", pkgbuild_path);
    }

    // Use bash to source the PKGBUILD and print the variables
    let output = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source '{}' 2>/dev/null && echo \"$pkgver\" && echo \"$pkgrel\"",
            pkgbuild_path
        ))
        .output()
        .context("Failed to execute bash to parse PKGBUILD")?;

    if !output.status.success() {
        anyhow::bail!("Failed to source PKGBUILD at: {}", pkgbuild_path);
    }

    let stdout =
        String::from_utf8(output.stdout).context("Failed to parse bash output as UTF-8")?;

    let mut lines = stdout.lines();

    let pkgver = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("pkgver not found in PKGBUILD"))?
        .trim()
        .to_string();

    let pkgrel = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("pkgrel not found in PKGBUILD"))?
        .trim()
        .to_string();

    if pkgver.is_empty() {
        anyhow::bail!("pkgver is empty in PKGBUILD");
    }

    if pkgrel.is_empty() {
        anyhow::bail!("pkgrel is empty in PKGBUILD");
    }

    Ok(PackageVersion { pkgver, pkgrel })
}

/// Simple regex-based parser as a fallback (less reliable but doesn't require bash)
/// Only handles simple variable assignments
pub fn parse_version_simple(pkgbuild_path: &str) -> Result<PackageVersion> {
    let content = fs::read_to_string(pkgbuild_path)
        .context(format!("Failed to read PKGBUILD at {}", pkgbuild_path))?;

    let mut pkgver = None;
    let mut pkgrel = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // Parse simple assignments
        if line.starts_with("pkgver=") {
            pkgver = Some(extract_value(line, "pkgver="));
        } else if line.starts_with("pkgrel=") {
            pkgrel = Some(extract_value(line, "pkgrel="));
        }
    }

    let pkgver = pkgver.ok_or_else(|| anyhow::anyhow!("pkgver not found in PKGBUILD"))?;
    let pkgrel = pkgrel.ok_or_else(|| anyhow::anyhow!("pkgrel not found in PKGBUILD"))?;

    Ok(PackageVersion { pkgver, pkgrel })
}

/// Extracts value from a simple bash variable assignment
fn extract_value(line: &str, prefix: &str) -> String {
    let value = line[prefix.len()..].trim();

    // Remove quotes if present
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

/// Extracts the package name from a PKGBUILD
pub fn parse_pkgname(pkgbuild_path: &str) -> Result<String> {
    let output = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source '{}' 2>/dev/null && echo \"$pkgname\"",
            pkgbuild_path
        ))
        .output()
        .context("Failed to execute bash to parse PKGBUILD")?;

    if !output.status.success() {
        anyhow::bail!("Failed to source PKGBUILD at: {}", pkgbuild_path);
    }

    let stdout =
        String::from_utf8(output.stdout).context("Failed to parse bash output as UTF-8")?;

    let pkgname = stdout.trim().to_string();

    if pkgname.is_empty() {
        anyhow::bail!("pkgname is empty in PKGBUILD");
    }

    Ok(pkgname)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_version_simple_basic() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "pkgver=1.2.3").unwrap();
        writeln!(file, "pkgrel=1").unwrap();

        let result = parse_version_simple(file.path().to_str().unwrap()).unwrap();
        assert_eq!(result.pkgver, "1.2.3");
        assert_eq!(result.pkgrel, "1");
        assert_eq!(result.to_string(), "1.2.3-1");
    }

    #[test]
    fn test_parse_version_simple_with_quotes() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "pkgver=\"1.2.3\"").unwrap();
        writeln!(file, "pkgrel='1'").unwrap();

        let result = parse_version_simple(file.path().to_str().unwrap()).unwrap();
        assert_eq!(result.pkgver, "1.2.3");
        assert_eq!(result.pkgrel, "1");
    }

    #[test]
    fn test_parse_version_simple_with_comments() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# This is a comment").unwrap();
        writeln!(file, "pkgver=1.2.3").unwrap();
        writeln!(file, "# Another comment").unwrap();
        writeln!(file, "pkgrel=1").unwrap();

        let result = parse_version_simple(file.path().to_str().unwrap()).unwrap();
        assert_eq!(result.pkgver, "1.2.3");
        assert_eq!(result.pkgrel, "1");
    }

    #[test]
    fn test_parse_version_nonexistent_file() {
        let result = parse_version_simple("/nonexistent/PKGBUILD");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_value() {
        assert_eq!(extract_value("pkgver=1.2.3", "pkgver="), "1.2.3");
        assert_eq!(extract_value("pkgver=\"1.2.3\"", "pkgver="), "1.2.3");
        assert_eq!(extract_value("pkgver='1.2.3'", "pkgver="), "1.2.3");
        assert_eq!(extract_value("pkgver=  1.2.3  ", "pkgver="), "1.2.3");
    }
}
