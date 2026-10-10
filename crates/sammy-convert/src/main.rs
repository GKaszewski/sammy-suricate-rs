mod extract;
mod glb;
mod images;
mod level;
mod model;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Converts Sammy Suricate (Acknex A4) data into modern formats.
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Unpack a WRS archive into a directory.
    Extract { wrs: PathBuf, out: PathBuf },
    /// Convert PCX images to PNG.
    Images { out: PathBuf, files: Vec<PathBuf> },
    /// Convert WMB levels to glTF (.glb).
    Level { out: PathBuf, files: Vec<PathBuf> },
    /// Convert MDL models to glTF (.glb).
    Model { out: PathBuf, files: Vec<PathBuf> },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Extract { wrs, out } => extract::run(&wrs, &out),
        Cmd::Images { out, files } => images::run(&out, &files),
        Cmd::Level { out, files } => level::run(&out, &files),
        Cmd::Model { out, files } => model::run(&out, &files),
    }
}
