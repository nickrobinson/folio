//! Build tooling for the Folio project.
//!
//! Usage:
//!   cargo xtask <command>
//!
//! Commands:
//!   publish        Publish all crates to crates.io in dependency order
//!   test-oracle    Run oracle comparison tests
//!   gen-bindings   Generate UniFFI language bindings
//!   corpus-stats   Show statistics about the test corpus

use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    let task = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match task {
        "publish" => publish(),
        "test-oracle" => test_oracle(),
        "gen-bindings" => gen_bindings(),
        "corpus-stats" => corpus_stats(),
        _ => print_help(),
    }
}

fn publish() {
    // Crates must be published leaf-first (dependencies before dependents).
    let crates = [
        "folio-core",
        "folio-filters",
        "folio-cos",
        "folio-doc",
        "folio-content",
        "folio-font",
        "folio-text",
        "folio-image",
        "folio-annot",
        "folio-forms",
        "folio-nav",
        "folio",
    ];

    let dry_run = env::args().any(|a| a == "--dry-run");

    for (i, krate) in crates.iter().enumerate() {
        if !dry_run && crate_already_published(krate) {
            println!("Skipping {krate} ({}/{}) — already published", i + 1, crates.len());
            continue;
        }

        println!("Publishing {krate} ({}/{})...", i + 1, crates.len());

        let mut args = vec!["publish", "-p", krate];
        if dry_run {
            args.push("--dry-run");
            args.push("--allow-dirty");
        }

        let status = Command::new("cargo")
            .args(&args)
            .status()
            .expect("Failed to run cargo publish");

        if !status.success() {
            eprintln!("Failed to publish {krate}");
            std::process::exit(1);
        }

        // Wait for crates.io index to update before publishing the next crate.
        if !dry_run && i < crates.len() - 1 {
            println!("Waiting for crates.io index to update...");
            thread::sleep(Duration::from_secs(15));
        }
    }

    println!("All crates published successfully!");
}

fn crate_already_published(name: &str) -> bool {
    // Get the local version from cargo pkgid (outputs: path+file:///...#0.0.1)
    let pkgid = Command::new("cargo")
        .args(["pkgid", "-p", name])
        .output()
        .expect("Failed to run cargo pkgid");
    let pkgid_str = String::from_utf8_lossy(&pkgid.stdout);
    let version = match pkgid_str.trim().rsplit_once('#') {
        Some((_, v)) => v.to_string(),
        None => return false,
    };

    // Check if this version exists on crates.io
    let output = Command::new("cargo")
        .args(["search", name, "--limit", "1"])
        .output()
        .expect("Failed to run cargo search");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // cargo search output: `folio-core = "0.0.1"    # description`
    stdout.lines().any(|line| {
        line.starts_with(&format!("{name} ")) && line.contains(&format!("\"{version}\""))
    })
}

fn test_oracle() {
    println!("Running oracle comparison tests...");
    let status = Command::new("cargo")
        .args(["test", "-p", "folio-oracle", "--", "--nocapture"])
        .status()
        .expect("Failed to run cargo test");

    if !status.success() {
        std::process::exit(1);
    }
}

fn gen_bindings() {
    println!("UniFFI binding generation requires uniffi-bindgen.");
    println!("Install with: cargo install uniffi_bindgen");
    println!(
        "Then run: uniffi-bindgen generate crates/folio-uniffi/src/folio.udl --language swift --out-dir bindings/swift"
    );
    println!(
        "     and: uniffi-bindgen generate crates/folio-uniffi/src/folio.udl --language kotlin --out-dir bindings/kotlin"
    );
}

fn corpus_stats() {
    let corpus_dir = "tests/corpus";
    let mut pdf_count = 0;
    let mut total_size: u64 = 0;

    if let Ok(entries) = std::fs::read_dir(corpus_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "pdf") {
                pdf_count += 1;
                if let Ok(meta) = path.metadata() {
                    total_size += meta.len();
                }
            }
        }
    }

    println!("Test corpus: {}", corpus_dir);
    println!("  PDF files: {}", pdf_count);
    println!("  Total size: {:.1} MB", total_size as f64 / 1_048_576.0);
}

fn print_help() {
    println!("Folio build tooling");
    println!();
    println!("Usage: cargo xtask <command>");
    println!();
    println!("Commands:");
    println!("  publish        Publish all crates to crates.io in dependency order");
    println!("                 Use --dry-run to validate without uploading");
    println!("  test-oracle    Run oracle comparison tests");
    println!("  gen-bindings   Generate UniFFI language bindings");
    println!("  corpus-stats   Show test corpus statistics");
}
