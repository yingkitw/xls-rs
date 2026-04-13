//! xls-rs - A CLI tool and MCP server for reading, writing, and converting spreadsheet files
//!
//! Supports CSV, Excel (xlsx/xls), ODS, Parquet, and Avro formats with formula evaluation.

#![allow(dead_code)] // Modules expose APIs for library use

use anyhow::Result;
use clap::Parser;
mod cli;

use cli::{Cli, CommandHandler, DefaultCommandHandler};

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::runtime::init(cli::runtime::CliRuntime {
        config_path: cli.config.clone(),
        quiet: cli.quiet,
        verbose: cli.verbose,
        overwrite: cli.overwrite,
    });
    let handler = DefaultCommandHandler::new();

    handler.handle(cli.command)
}
