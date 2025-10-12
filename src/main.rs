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

        /// Return full paths instead of package names (e.g., "packages/niri" instead of "niri")
        #[arg(short, long)]
        paths: bool,
    },

    /// List all packages in the repository
    ListPackages {
        /// Git repository path
        #[arg(short, long, default_value = ".")]
        repo_path: String,

        /// Show version information from PKGBUILD
        #[arg(short, long)]
        verbose: bool,

        /// Show full paths instead of package names
        #[arg(short, long)]
        paths: bool,
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
            paths,
        } => {
            let packages = if all {
                // Return all packages
                package::find_all_packages(&repo_path)?
            } else {
                // Return only changed packages
                let changed_names = git::detect_changed_packages(&repo_path, base_ref.as_deref())?;
                let all_packages = package::find_all_packages(&repo_path)?;

                // Filter packages to only those that changed
                all_packages
                    .into_iter()
                    .filter(|p| changed_names.contains(&p.name))
                    .collect()
            };

            // Extract either names or paths
            let output: Vec<String> = packages
                .iter()
                .map(|p| {
                    if paths {
                        p.path.clone()
                    } else {
                        p.name.clone()
                    }
                })
                .collect();

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                "space" => {
                    println!("{}", output.join(" "));
                }
                _ => {
                    anyhow::bail!("Unknown format: {}", format);
                }
            }
        }

        Commands::ListPackages {
            repo_path,
            verbose,
            paths,
        } => {
            let packages = package::find_all_packages(&repo_path)?;

            for pkg in packages {
                let identifier = if paths { &pkg.path } else { &pkg.name };

                if verbose {
                    if let Ok(version) = pkgbuild::parse_version(&pkg.pkgbuild_path) {
                        println!("{}: {}", identifier, version);
                    } else {
                        println!("{}: <version unknown>", identifier);
                    }
                } else {
                    println!("{}", identifier);
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
