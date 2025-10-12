use anyhow::Result;
use clap::{Parser, Subcommand};

mod git;
mod package;
mod pkgbuild;

#[derive(Parser)]
#[command(name = "syspac")]
#[command(about = "Artix Linux package repository management tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect packages that have changed between commits
    DetectChanges {
        /// Git repository path
        #[arg(short, long, default_value = ".")]
        repo_path: String,

        /// Base commit/ref to compare against (defaults to HEAD^)
        #[arg(short, long)]
        base_ref: Option<String>,

        /// Output format: space-separated list or JSON
        #[arg(short, long, default_value = "space")]
        format: String,

        /// Return all packages regardless of changes (for full rebuild)
        #[arg(short, long)]
        all: bool,
    },

    /// List all packages in the repository
    ListPackages {
        /// Git repository path
        #[arg(short, long, default_value = ".")]
        repo_path: String,

        /// Show version information from PKGBUILD
        #[arg(short, long)]
        verbose: bool,
    },

    /// Get package version from PKGBUILD
    PackageVersion {
        /// Path to PKGBUILD or package directory
        path: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::DetectChanges {
            repo_path,
            base_ref,
            format,
            all,
        } => {
            let changes = if all {
                // Return all packages
                let packages = package::find_all_packages(&repo_path)?;
                packages.iter().map(|p| p.name.clone()).collect()
            } else {
                // Return only changed packages
                git::detect_changed_packages(&repo_path, base_ref.as_deref())?
            };

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&changes)?);
                }
                "space" => {
                    println!("{}", changes.join(" "));
                }
                _ => {
                    anyhow::bail!("Unknown format: {}", format);
                }
            }
        }

        Commands::ListPackages { repo_path, verbose } => {
            let packages = package::find_all_packages(&repo_path)?;

            for pkg in packages {
                if verbose {
                    if let Ok(version) = pkgbuild::parse_version(&pkg.pkgbuild_path) {
                        println!("{}: {}", pkg.name, version);
                    } else {
                        println!("{}: <version unknown>", pkg.name);
                    }
                } else {
                    println!("{}", pkg.name);
                }
            }
        }

        Commands::PackageVersion { path } => {
            let pkgbuild_path = if path.ends_with("PKGBUILD") {
                path
            } else {
                format!("{}/PKGBUILD", path.trim_end_matches('/'))
            };

            let version = pkgbuild::parse_version(&pkgbuild_path)?;
            println!("{}", version);
        }
    }

    Ok(())
}
