//! Build tooling for the Folio project.
//!
//! Usage:
//!   cargo xtask <command>
//!
//! Commands:
//!   test-oracle    Run oracle comparison tests
//!   gen-bindings   Generate UniFFI language bindings
//!   corpus-stats   Show statistics about the test corpus

use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let task = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match task {
        "test-oracle" => test_oracle(),
        "gen-bindings" => gen_bindings(),
        "corpus-stats" => corpus_stats(),
        _ => print_help(),
    }
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
    println!("  test-oracle    Run oracle comparison tests");
    println!("  gen-bindings   Generate UniFFI language bindings");
    println!("  corpus-stats   Show test corpus statistics");
}
