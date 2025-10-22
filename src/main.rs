use std::path::Path;

use clap::Parser;
use commands::{Commands, Game, Operation, create, extract};

use crate::commands::ChecksumValidation;

mod commands;

fn main() -> anyhow::Result<()> {
    let cmd = match commands::Commands::try_parse() {
        Ok(cmd) => cmd,
        Err(e) => {
            // a simple hack to allow drag and droping files to the program

            let mut hvp = None;
            let mut folder = None;

            for arg in std::env::args().skip(1) {
                let path = Path::new(&arg);
                if arg.to_lowercase().ends_with(".hvp") && path.is_file() {
                    hvp = Some(path.to_path_buf());
                } else if path.is_dir() {
                    folder = Some(path.to_path_buf());
                }
            }

            let Some(hvp) = hvp else { e.exit() };

            let operation = match folder {
                Some(input_folder) => Operation::Create(create::Commands {
                    input_hvp: hvp,
                    input_folder,
                    output: None,
                    skip_compression: false,
                    checksum_validation: ChecksumValidation::Prompt,
                    update_all_files: false,
                    generate_anyway: false,
                }),
                None => Operation::Extract(extract::Commands {
                    input: hvp,
                    output_folder: None,
                    checksum_validation: ChecksumValidation::Prompt,
                }),
            };

            Commands {
                operation,
                game: Game::Auto,
            }
        }
    };

    cmd.start()
}
