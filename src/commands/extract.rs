use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use anstream::{print, println};
use anyhow::Context;
use clap::{Parser, ValueHint};
use hvp_archive::{
    Game,
    archive::{Archive, Obscure2NameMap, Options, entry::DecompressError},
    provider::ArchiveProvider,
};
use indicatif::ParallelProgressIterator;
use owo_colors::OwoColorize;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use super::{ChecksumValidation, HASHES_FILE, load_name_maps, utils};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub struct Commands {
    /// path to input hvp archive
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    pub input: PathBuf,
    /// output folder, if empty a folder with the same name as input will be used
    #[arg(value_hint = ValueHint::DirPath)]
    pub output_folder: Option<PathBuf>,
    /// validate checksums of the files
    #[arg(long, short = 's', default_value_t = ChecksumValidation::Yes, value_enum, required = false)]
    pub checksum_validation: ChecksumValidation,
}

impl Commands {
    /// handle the user command
    pub fn start(self, provider: ArchiveProvider) -> anyhow::Result<()> {
        let obscure2_names = match provider.game() {
            Game::Obscure2 => match load_name_maps().context("failed to load name maps")? {
                Some(names) => names,
                None => {
                    println!(
                        "{} failed to load obscure2 (or alone in the dark 2008) name maps because no hash file was found",
                        "[!]".yellow()
                    );

                    Obscure2NameMap::default()
                }
            },
            _ => Obscure2NameMap::default(), // we don't need to load name map for any other game
        };

        let archive = Archive::new_with_options(
            &provider,
            Options {
                obscure2_names,
                rebuild_skip_compression: false,
            },
        );

        utils::print_metadata(archive.metadata());

        if matches!(
            self.checksum_validation,
            ChecksumValidation::Yes | ChecksumValidation::Prompt
        ) {
            println!("{} validating entries checksum", "[+]".green());
            if !archive.entries_checksum_match() {
                let mut should_exit = true;

                if self.checksum_validation == ChecksumValidation::Prompt {
                    print!(
                        "{} checksum mismatch, continue anyway? [y/n]: ",
                        "[!]".yellow()
                    );
                    anstream::stdout().flush()?;
                    let input = utils::prompt()?.to_lowercase();
                    match input.as_str() {
                        "y" => should_exit = false,
                        "n" => should_exit = true,
                        _ => {
                            println!("{} invalid input: '{}'", "[!]".red(), input);
                            should_exit = true;
                        }
                    }
                }

                if should_exit {
                    anyhow::bail!(
                        "archive entries checksum doesn't match, maybe the archive is broken?"
                    );
                }
            }
        }

        let output = self
            .output_folder
            .unwrap_or_else(|| self.input.with_extension(""));

        println!("{} output folder: {}", "[+]".green(), output.display());

        if !output.is_dir() {
            println!("{} creating output folder", "[+]".green());
            std::fs::create_dir_all(&output).context("failed to create output folder")?;
        }

        // we do this so we don't have to join output dir with entry path each time
        println!(
            "{} changing working directory to output path",
            "[+]".green()
        );
        std::env::set_current_dir(output)
            .context("failed to change working directory to output path")?;

        // we collect everything in a vector so rayon can access them in random order
        let files: Vec<_> = archive.files().collect();

        println!("{} starting the extraction", "[+]".green());

        let pb = utils::progress_bar(files.len() as _);

        let hashes: ahash::HashMap<u32, u32> = files
            .into_par_iter()
            .map_with(pb.clone(), |pb, entry| {
                let path_crc32 = crc32fast::hash(entry.path.display().to_string().as_bytes());

                // create output dir if not exist
                let path = entry.path.with_file_name("");
                if !path.is_dir() {
                    std::fs::create_dir_all(path)?;
                }

                // not the best way, but right now I really don't want to deal with custom error type
                let bytes = entry.get_bytes()?;

                // write to disk
                std::fs::write(&entry.path, &bytes)?;

                pb.set_message(entry.path.display().to_string());

                let content_crc32 = crc32fast::hash(&bytes);

                Ok((path_crc32, content_crc32))
            })
            .progress_with(pb.clone())
            .collect::<Result<_, ExtractError>>()
            .context("extraction failed")?;

        pb.finish_with_message(
            "extraction finished"
                .if_supports_color(owo_colors::Stream::Stdout, |t| t.green())
                .to_string(),
        );

        println!("{} extraction finished", "[+]".green());
        print!("{} writing hashes.json to output folder", "[+]".green());

        let writer =
            BufWriter::new(File::create(HASHES_FILE).context("failed to create hashes.json file")?);

        serde_json::to_writer_pretty(writer, &hashes).context("failed to serialize file hashes")?;

        println!(": Done");

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
enum ExtractError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Decompress(#[from] DecompressError),
}
