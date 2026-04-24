# PPC Metacompiler

A metacompiler for C that processes preprocessed `.i` files, performing template extraction, solving, and code generation.

## Building

```bash
cargo build
```

The executable will be located at target/debug/PPC_template_metacompiler (or the crate name as defined in Cargo.toml).

## Usage
```bash
PPC_template_metacompiler --input <INPUT.i> --output <OUTPUT.i> [--debug]
```
## Options
Option	Description
-i, --input <FILE>	Input .i file (preprocessed C)
-o, --output <FILE>	Output .i file
-d, --debug	Print token streams to stdout for debugging
## Example
```bash
# Process a file
PPC_template_metacompiler --input example.i --output processed.i
```
```# With debug output
PPC_template_metacompiler -i example.i -o out.i -debug
```
## Notes
If input and output paths are the same, the tool creates a temporary file and a backup (.bak) to avoid data loss.
Debug mode shows three tokenization stages: initial, after template extraction, and after template solving.
