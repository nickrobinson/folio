//! List all form fields in a PDF with their types and values.
//!
//! Usage:
//!   cargo run -p folio --example list_forms -- path/to/form.pdf

use folio::prelude::*;
use std::env;

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: list_forms <path-to-pdf>");
            std::process::exit(1);
        }
    };

    let data = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {}", path, e);
        std::process::exit(1);
    });

    let mut doc = CosDoc::open(data).unwrap_or_else(|e| {
        eprintln!("Cannot open PDF: {}", e);
        std::process::exit(1);
    });

    let fields = AcroForm::get_fields(&mut doc).unwrap_or_else(|e| {
        eprintln!("Cannot read form fields: {}", e);
        std::process::exit(1);
    });

    if fields.is_empty() {
        println!("No form fields found in {}", path);
        return;
    }

    println!("{} form fields in {}:\n", fields.len(), path);
    println!("{:<30} {:<12} {:<10} {}", "Name", "Type", "Flags", "Value");
    println!("{}", "-".repeat(80));

    for field in &fields {
        let type_str = match field.field_type() {
            FieldType::Text => "Text",
            FieldType::CheckBox => "CheckBox",
            FieldType::Radio => "Radio",
            FieldType::Button => "Button",
            FieldType::Choice => "Choice",
            FieldType::Signature => "Signature",
            FieldType::Unknown => "Unknown",
        };

        let mut flag_parts = Vec::new();
        let flags = field.flags();
        if flags.contains(FieldFlags::READ_ONLY) {
            flag_parts.push("RO");
        }
        if flags.contains(FieldFlags::REQUIRED) {
            flag_parts.push("REQ");
        }
        if flags.contains(FieldFlags::MULTILINE) {
            flag_parts.push("ML");
        }
        if flags.contains(FieldFlags::PASSWORD) {
            flag_parts.push("PW");
        }
        if flags.contains(FieldFlags::COMBO) {
            flag_parts.push("CMB");
        }
        let flags_str = if flag_parts.is_empty() {
            "-".to_string()
        } else {
            flag_parts.join(",")
        };

        let value = field.value().unwrap_or_else(|| "(empty)".into());
        let value_display = if value.len() > 30 {
            format!("{}...", &value[..27])
        } else {
            value
        };

        println!(
            "{:<30} {:<12} {:<10} {}",
            truncate(field.name(), 29),
            type_str,
            flags_str,
            value_display
        );

        // Show options for choice fields
        let options = field.options();
        if !options.is_empty() {
            for opt in options.iter().take(5) {
                println!("  option: {}", opt);
            }
            if options.len() > 5 {
                println!("  ... ({} more options)", options.len() - 5);
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.to_string()
    }
}
