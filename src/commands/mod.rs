use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anstream::println;
use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use hvp_archive::{archive::Obscure2NameMap, provider::ArchiveProvider};
use owo_colors::OwoColorize;

pub mod create;
#[cfg(feature = "dump")]
mod dump;
pub mod extract;
mod utils;

const HASHES_FILE: &str = "hashes.json";

#[derive(Parser)]
#[command(
    name = "Obscure hvp tool",
    author,
    version,
    arg_required_else_help = true
)]
pub struct Commands {
    #[command(subcommand)]
    pub operation: Operation,
    /// What game is the archive from
    #[arg(long, short = 'g', default_value_t = Game::Auto, value_enum, global = true)]
    pub game: Game,
}

impl Commands {
    /// handle the user command
    pub fn start(self) -> anyhow::Result<()> {
        let hvp_path = self.operation.input_hvp_path();
        let file = File::open(hvp_path).context("failed to open hvp archive")?;

        let provider = ArchiveProvider::new(file, self.game.into())
            .context("failed to load input hvp archive")?;

        match self.operation {
            #[cfg(feature = "dump")]
            Operation::Dump(commands) => commands.start(provider),
            Operation::Extract(commands) => commands.start(provider),
            Operation::Create(commands) => commands.start(provider),
        }
    }
}

#[derive(Subcommand)]
pub enum Operation {
    /// dump hvp archive TOC as json
    #[cfg(feature = "dump")]
    Dump(dump::Commands),
    /// extract files from hvp archive
    Extract(extract::Commands),
    /// create a new hvp archive based on extracted data and original archive
    Create(create::Commands),
}

impl Operation {
    pub fn input_hvp_path(&self) -> &Path {
        match self {
            #[cfg(feature = "dump")]
            Operation::Dump(cmd) => &cmd.input,
            Operation::Extract(cmd) => &cmd.input,
            Operation::Create(cmd) => &cmd.input_hvp,
        }
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, Default)]
pub enum Game {
    /// auto detect the game based on input hvp
    #[default]
    Auto,
    /// Obscure 1 game
    Obscure1,
    /// Obscure 2 game (also work with alone in the dark 2008)
    Obscure2,
    /// Final Exam game
    FinalExam,
}

impl From<Game> for Option<hvp_archive::Game> {
    fn from(value: Game) -> Self {
        match value {
            Game::Auto => None,
            Game::Obscure1 => Some(hvp_archive::Game::Obscure1),
            Game::Obscure2 => Some(hvp_archive::Game::Obscure2),
            Game::FinalExam => Some(hvp_archive::Game::FinalExam),
        }
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ChecksumValidation {
    /// validate checksums and don't extract files if checksums don't match
    #[default]
    Yes,
    /// skip checksum validation
    No,
    /// check checksums, in case of checksum mismatch, ask user if they want to continue
    Prompt,
}

fn load_name_maps() -> std::io::Result<Option<Obscure2NameMap>> {
    let path = Path::new("hashes");

    println!(
        "{} loading name maps from {} directory",
        "[?]".green(),
        path.display(),
    );

    if !path.is_dir() {
        return Ok(None);
    }

    let mut names = Vec::new();

    let dir = path.read_dir()?;
    for entry in dir {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || path.extension().unwrap_or_default() != "txt" {
            continue;
        }

        println!("{} loading name map from {}", "[?]".green(), path.display(),);

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
        names.extend(lines);
    }

    if names.is_empty() {
        return Ok(None);
    }

    Ok(Some(Obscure2NameMap::new(names)))
}
