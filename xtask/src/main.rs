//! xtask: Build tooling for hipfire-arwaky
//!
//! Subcommands:
//!   patch      - Copy symlinked crates to local-patched/ and apply patches
//!   clean      - Remove local-patched/ (regenerated on next patch)
//!   kernel-gen - Generate kernel dispatch code (future)

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fs_extra::dir::{copy, CopyOptions};
use glob::glob;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Parser)]
#[command(name = "xtask", version, about = "hipfire-arwaky build tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy crates from symlinks to local-patched/ and apply .patch files
    Patch {
        /// Only patch specific crate(s) (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        crates: Option<Vec<String>>,
        /// Force re-copy even if local-patched exists
        #[arg(short, long)]
        force: bool,
        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove local-patched/ directory
    Clean {
        /// Confirm without prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// List crates that would be patched
    List,
    /// Generate kernel dispatch (placeholder for future)
    KernelGen {
        #[arg(short, long)]
        arch: Option<String>,
    },
}

// ── Essential crates for Qwen3.5 + RDNA2 ──────────────────────────────
const ESSENTIAL_CRATES: &[&str] = &[
    "hip-bridge",
    "rdna-compute",
    "hipfire-dispatch",
    "hipfire-runtime",
    "hipfire-arch-qwen35",
    "hipfire-quantize",
    "hipfire-detect",
    "hipfire-atlas",
    "hipfire-tui",
];

const PATCHES_DIR: &str = "patches";
const LOCAL_PATCHED_DIR: &str = "local-patched";
const SYMLINK_DIR: &str = "crates";

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Patch { crates, force, dry_run } => {
            let target_crates = crates.unwrap_or_else(|| ESSENTIAL_CRATES.iter().map(|s| s.to_string()).collect());
            run_patch(&target_crates, force, dry_run)
        }
        Commands::Clean { yes } => run_clean(yes),
        Commands::List => run_list(),
        Commands::KernelGen { arch } => run_kernel_gen(arch),
    }
}

fn run_patch(crates: &[String], force: bool, dry_run: bool) -> Result<()> {
    let root = std::env::current_dir()?;
    let symlink_dir = root.join(SYMLINK_DIR);
    let patched_dir = root.join(LOCAL_PATCHED_DIR);
    let patches_dir = root.join(PATCHES_DIR);

    if dry_run {
        println!("🔍 DRY RUN - would patch crates: {:?}", crates);
        for krate in crates {
            let src = symlink_dir.join(krate);
            let dst = patched_dir.join(krate);
            let patch_dir = patches_dir.join(krate);
            println!("  {} -> {}", src.display(), dst.display());
            if patch_dir.exists() {
                for patch in glob(&format!("{}/*.patch", patch_dir.display()))? {
                    println!("    apply: {}", patch?.display());
                }
            }
        }
        return Ok(());
    }

    fs::create_dir_all(&patched_dir)?;

    for krate_name in crates {
        println!("📦 Processing crate: {}", krate_name);

        let src = symlink_dir.join(&krate_name);
        let dst = patched_dir.join(&krate_name);
        let patch_dir = patches_dir.join(&krate_name);

        // Validate source exists
        if !src.exists() {
            anyhow::bail!("Source crate not found: {}", src.display());
        }

        // Copy crate to local-patched (dereference symlinks)
        if dst.exists() {
            if force {
                println!("  🔄 Force: removing existing {}", dst.display());
                fs::remove_dir_all(&dst)?;
            } else {
                println!("  ⏭️  Skipping {} (exists, use --force to overwrite)", krate_name);
                continue;
            }
        }

        println!("  📋 Copying {} -> {}", src.display(), dst.display());
        let mut opts = CopyOptions::new();
        opts.copy_inside = false;  // copy directory contents, not the directory itself
        opts.content_only = true;
        opts.overwrite = true;
        copy(&src, &dst, &opts).with_context(|| format!("Failed to copy {}", krate_name))?;

        // Apply patches if any
        if patch_dir.exists() {
            let patches: Vec<PathBuf> = glob(&format!("{}/*.patch", patch_dir.display()))?
                .filter_map(Result::ok)
                .collect();

            if !patches.is_empty() {
                println!("  🩹 Applying {} patches from {}", patches.len(), patch_dir.display());
                for patch_file in &patches {
                    println!("    → {}", patch_file.file_name().unwrap().to_string_lossy());
                    let status = Command::new("git")
                        .args(["apply", "--directory", &krate_name, &patch_file.to_string_lossy()])
                        .current_dir(&patched_dir)
                        .status()
                        .with_context(|| format!("git apply failed for {}", patch_file.display()))?;

                    if !status.success() {
                        anyhow::bail!("git apply failed for patch: {}", patch_file.display());
                    }
                }
            } else {
                println!("  ℹ️  No patches found in {}", patch_dir.display());
            }
        } else {
            println!("  ℹ️  No patches directory for {}", krate_name);
        }
    }

    println!("✅ Patch complete. Crates ready in {}", patched_dir.display());
    println!("   Run 'cargo build' to compile with [patch] redirect.");
    Ok(())
}

fn run_clean(yes: bool) -> Result<()> {
    let root = std::env::current_dir()?;
    let patched_dir = root.join(LOCAL_PATCHED_DIR);

    if !patched_dir.exists() {
        println!("ℹ️  local-patched/ does not exist");
        return Ok(());
    }

    if !yes {
        print!("⚠️  Delete {}? [y/N] ", patched_dir.display());
        use std::io::{stdin, stdout, Write};
        stdout().flush()?;
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    fs::remove_dir_all(&patched_dir)?;
    println!("🗑️  Removed {}", patched_dir.display());
    Ok(())
}

fn run_list() -> Result<()> {
    println!("📋 Essential crates for Qwen3.5 + RDNA2 (gfx1030/1031):");
    for krate in ESSENTIAL_CRATES {
        let symlink = Path::new(SYMLINK_DIR).join(krate);
        let patched = Path::new(LOCAL_PATCHED_DIR).join(krate);
        let patches = Path::new(PATCHES_DIR).join(krate);

        let symlink_status = if symlink.exists() { "✅" } else { "❌" };
        let patched_status = if patched.exists() { "✅" } else { "⏳" };
        let patches_status = if patches.exists() { "🩹" } else { "📂" };

        println!("  {} {}  {}  {}  {}", symlink_status, patched_status, patches_status, krate,
            if patches.exists() { format!("({} patches)", glob(&format!("{}/*.patch", patches.display()))?.count()) } else { String::new() });
    }
    Ok(())
}

fn run_kernel_gen(arch: Option<String>) -> Result<()> {
    let arch = arch.unwrap_or_else(|| "gfx1030".to_string());
    println!("🔧 Kernel generation for {} (not yet implemented)", arch);
    println!("   Future: generate dispatch code from kernels/src/ for target arch");
    Ok(())
}