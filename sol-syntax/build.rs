use std::{fmt::Display, fs::File, io::Write, path::Path, process::Command};

use rust_format::{Formatter, RustFmt};
use type_sitter_gen::tree_sitter;

fn main() {
    regenerate_tree_sitter_grammar();

    if std::env::var("NO_REBUILD_TYPE_SITTER").is_err() {
        regenerate_node_types();
    }

    println!("cargo:rerun-if-changed=../tree-sitter-sol/grammar.js");
}

/// Regenerate tree-sitter grammar
fn regenerate_tree_sitter_grammar() {
    Command::new("tree-sitter")
        .current_dir(Path::new("../tree-sitter-sol"))
        .arg("generate")
        .status()
        .expect("failed to generate tree-sitter grammar");

    Command::new("tree-sitter")
        .current_dir(Path::new("../tree-sitter-sol"))
        .arg("build-wasm")
        .status()
        .expect("failed to generate tree-sitter grammar for wasm");
}

/// Regenerates the `src/generated/sol.rs` file from the `src/node-types.json`
fn regenerate_node_types() {
    let input_path = Path::new("../tree-sitter-sol/src/node-types.json");
    let target = Path::new("src/generated/node_types.rs");
    let node_types = type_sitter_gen::generate_nodes(input_path, &tree_sitter())
        .expect("failed to generate nodes");
    clear_output_files(target);
    write_rust_file(target, node_types);
}

/// Clears the output file so we don't write a directory or an invalid file
/// when the build script fails.
fn clear_output_files(target: &Path) {
    if target.is_dir() {
        std::fs::remove_dir_all(target).expect("failed to remove directory");
    }
    if target.exists() && target.is_file() {
        std::fs::remove_file(target).expect("failed to remove file");
    }
}

/// Writes the generated Rust code to the output file, formatting it
/// with rustfmt.
fn write_rust_file(path: &Path, contents: impl Display) {
    let mut file = File::create(path).expect("failed to create file");
    let contents = RustFmt::new()
        .format_str(contents.to_string())
        .expect("failed to format file");
    write!(file, "{}", contents).expect("failed to write file");
}
