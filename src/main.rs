mod tokenizer;
mod cstandard;
mod template_solver;

mod template_extractor;
mod utils;
mod name_generator;
mod code_generator;

use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use tokenizer::{tokenize, TokenInfo};


/// Metacompiler for C language that processes .i files
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file path (.i file)
    #[arg(short, long)]
    input: String,

    /// Output file path (.i file)
    #[arg(short, long)]
    output: String,

    /// Print tokens to stdout for debugging
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("PPC Metacompiler - Processing file: {}", args.input);

    // Read input file
    let content = read_file(&args.input)
        .with_context(|| format!("Failed to read input file: {}", args.input))?;

    println!("Read {} bytes from input file", content.len());

    // Process content (currently just passes through)
    let processed_content = process_content(&content, args.debug);

    let processed_content = processed_content?;
    // Write output file
    write_file(&args.output, &processed_content, &args.input)
        .with_context(|| format!("Failed to write output file: {}", args.output))?;

    println!(
        "Successfully wrote {} bytes to output file",
        processed_content.len()
    );

    Ok(())
}

/// Reads content from a file
fn read_file(filename: &str) -> Result<String> {
    fs::read_to_string(filename).with_context(|| format!("Failed to read file: {}", filename))
}

/// Processes the file content (placeholder implementation)
fn process_content(content: &str, debug: bool) -> std::result::Result<String, anyhow::Error> {
    let tokens = tokenize(content)
        .with_context(|| "Failed to tokenize content")?;

    if debug {
        print_tokens(&tokens);
    }

    let (extracted_tokens, templates, struct_templates, additional_templates) = template_extractor::extract_templates(tokens)?;

    if debug {
        print_tokens(&extracted_tokens);
    }

    let solved_tokens = template_solver::solve_templates(templates, struct_templates, additional_templates, extracted_tokens)?;

    if debug {
        print_tokens(&solved_tokens);
    }

    Ok(code_generator::generate_code(solved_tokens))
}

/// Prints tokens for debugging
fn print_tokens(tokens: &[TokenInfo]) {
    println!("\n=== TOKENS ===");
    for (i, token_info) in tokens.iter().enumerate() {
        println!("{:4}: {:3}:{:3} - {:?}",
                 i,
                 token_info.line,
                 token_info.column,
                 token_info.token);
    }
    println!("=== END TOKENS ===\n");
}

/// Writes content to a file, handling the case where input and output are the same file
fn write_file(output_filename: &str, content: &str, input_filename: &str) -> Result<()> {
    if output_filename == input_filename {
        // If same file, use temporary file strategy to avoid corruption
        write_file_with_backup(output_filename, content)
    } else {
        // Different files, direct write is safe
        fs::write(output_filename, content)
            .with_context(|| format!("Failed to write to file: {}", output_filename))
    }
}

/// Writes content to a file using temporary file to prevent data loss
fn write_file_with_backup(filename: &str, content: &str) -> Result<()> {
    let temp_filename = format!("{}.tmp", filename);
    let backup_filename = format!("{}.bak", filename);

    // Write to temporary file first
    fs::write(&temp_filename, content)
        .with_context(|| format!("Failed to write to temporary file: {}", temp_filename))?;

    // If original file exists, create backup
    if fs::metadata(filename).is_ok() {
        fs::rename(filename, &backup_filename)
            .with_context(|| format!("Failed to create backup: {}", backup_filename))?;
    }

    // Move temporary file to final location
    fs::rename(&temp_filename, filename)
        .with_context(|| format!("Failed to move temporary file to: {}", filename))?;

    // Clean up backup file if it exists
    if fs::metadata(&backup_filename).is_ok() {
        let _ = fs::remove_file(&backup_filename);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_write_different_files() -> Result<()> {
        let input_file = NamedTempFile::new()?;
        let output_file = NamedTempFile::new()?;

        let test_content = "test content";
        fs::write(input_file.path(), test_content)?;

        let content = read_file(input_file.path().to_str().unwrap())?;
        write_file(
            output_file.path().to_str().unwrap(),
            &content,
            input_file.path().to_str().unwrap(),
        )?;

        let output_content = fs::read_to_string(output_file.path())?;
        assert_eq!(output_content, test_content);

        Ok(())
    }

    #[test]
    fn test_tokenizer_basic() -> Result<()> {
        let source = "int main() { return 42; }";
        let tokens = tokenize(source)?;
        assert!(!tokens.is_empty());
        Ok(())
    }
}
