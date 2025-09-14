#![feature(if_let_guard, string_remove_matches, str_as_str)]
#![forbid(unsafe_code, clippy::unwrap_used, clippy::expect_used)]

mod cli;
mod config;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::ParcelCLI;

#[cfg(not(target_os = "macos"))]
compile_error!("Parcel is currently only supported on macOS.");

fn main() -> ExitCode {
    let cli = ParcelCLI::parse();

    if let Err(e) = cli.run() {
        eprintln!("{}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
