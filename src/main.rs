#![feature(if_let_guard, string_remove_matches, str_as_str)]

mod cli;
mod config;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::ParcelCLI;

fn main() -> ExitCode {
    let cli = ParcelCLI::parse();

    if let Err(e) = cli.run() {
        eprintln!("{}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
