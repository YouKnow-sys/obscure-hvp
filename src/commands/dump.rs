use std::{fs::File, io::BufWriter, path::PathBuf};

use anstream::{print, println};
use anyhow::Context;
use clap::{Parser, ValueHint};
use hvp_archive::{
    archive::Archive,
    provider::{ArchiveProvider, RawArchive},
};
use owo_colors::OwoColorize;

use super::utils;

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub struct Commands {
    /// path to input hvp archive
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    pub input: PathBuf,
    /// output json file, if empty a json file with the same name of input hvp will be created
    pub output: Option<PathBuf>,
}

impl Commands {
    /// handle the user command
    pub fn start(self, provider: ArchiveProvider) -> anyhow::Result<()> {
        let archive = Archive::new(&provider);

        utils::print_metadata(archive.metadata());

        let output = self
            .output
            .unwrap_or_else(|| self.input.with_extension("json"));

        println!("{} output file: {}", "[+]".green(), output.display());
        print!("{} serializng entries to json", "[+]".green());

        let writer =
            BufWriter::new(File::create(output).context("failed to create output json file")?);

        match provider.raw_archive() {
            RawArchive::Obscure1(archive) => serde_json::to_writer_pretty(writer, &archive.entries),
            RawArchive::Obscure2(archive) => serde_json::to_writer_pretty(writer, &archive.entries),
        }
        .context("failed to serialize entries")?;

        println!(": Done");

        Ok(())
    }
}
