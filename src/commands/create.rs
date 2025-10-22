use std::{
    ffi::OsStr,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anstream::{print, println};
use anyhow::Context;
use clap::{Parser, ValueHint};
use hvp_archive::{
    archive::{Archive, Options, entry::UpdateKind, rebuild_progress::RebuildProgress},
    provider::ArchiveProvider,
};
use indicatif::{ParallelProgressIterator, ProgressBar};
use owo_colors::OwoColorize;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::commands::ChecksumValidation;

use super::{HASHES_FILE, load_obscure2_name_map, utils};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub struct Commands {
    /// path to input hvp archive
    #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
    pub input_hvp: PathBuf,
    /// path to folder of exported data
    #[arg(value_hint = ValueHint::DirPath, value_parser = utils::is_dir)]
    pub input_folder: PathBuf,
    /// output file, if empty a new file with the same name of input hvp will be created (+ new)
    pub output: Option<PathBuf>,
    /// skip compression of the files
    #[arg(long, short = 'c', default_value_t = false, required = false)]
    pub skip_compression: bool,
    /// validate checksums of the files
    #[arg(long, short = 's', default_value_t = ChecksumValidation::Yes, value_enum, required = false)]
    pub checksum_validation: ChecksumValidation,
    /// skip checking for modified files and just update all files
    #[arg(long, short = 'a', default_value_t = false, required = false)]
    pub update_all_files: bool,
    /// create archive even when no files changed
    #[arg(long, default_value_t = false, required = false)]
    pub generate_anyway: bool,
}

impl Commands {
    /// handle the user command
    pub fn start(self, provider: ArchiveProvider) -> anyhow::Result<()> {
        let mut archive = Archive::new_with_options(
            &provider,
            Options {
                obscure2_names: load_obscure2_name_map(),
                rebuild_skip_compression: self.skip_compression,
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

        let output = self.output.unwrap_or_else(|| {
            self.input_hvp.with_extension(
                self.input_hvp
                    .extension()
                    .and_then(OsStr::to_str)
                    .map(|e| format!("new.{e}"))
                    .unwrap_or("new".to_owned()),
            )
        });

        println!("{} output hvp archive: {}", "[+]".green(), output.display());

        let files = utils::list_files(&self.input_folder, true);

        if files.is_empty() && self.generate_anyway {
            anyhow::bail!("no file found in input folder")
        }

        let org_working_dir =
            std::env::current_dir().context("failed to get current working directory")?;

        // we do this so we don't have to join output dir with entry path each time
        println!(
            "{} changing working directory to input folder",
            "[+]".green()
        );
        std::env::set_current_dir(&self.input_folder)
            .context("failed to change working directory to output path")?;

        print!(
            "{} found {} files in input folder",
            "[+]".green(),
            files.len()
        );

        let files = if Path::new(HASHES_FILE).is_file() && !self.update_all_files {
            println!(". {}", "filtering based on modified files".blink().cyan());
            let txt = std::fs::read_to_string(HASHES_FILE).context("failed to read hashes.json")?;
            let hashes: ahash::HashMap<u32, u32> = serde_json::from_str(&txt).context(
                "failed to load file hashes from hashes.json, if you modified it just remove it",
            )?;

            let pb = utils::progress_bar(files.len() as _);

            let all_files_len = files.len();

            let hashed_files: ahash::HashMap<u32, (u32, PathBuf)> = files
                .into_par_iter()
                .map_with(pb.clone(), |pb, path| {
                    let bytes = std::fs::read(&path)?;
                    let path_str = path.display().to_string();

                    let name_crc32 = crc32fast::hash(path_str.as_bytes());
                    let content_crc32 = crc32fast::hash(&bytes);

                    pb.set_message(path_str);

                    Ok((name_crc32, (content_crc32, path)))
                })
                .progress_with(pb.clone())
                .collect::<std::io::Result<_>>()
                .context("failed to generate crc32 of files in input folder")?;

            pb.finish_with_message(
                "checking finished"
                    .if_supports_color(owo_colors::Stream::Stdout, |t| t.green())
                    .to_string(),
            );

            // to remove hashes.json
            let hashes_file = Path::new(HASHES_FILE);

            let filterd_files: Vec<PathBuf> = hashed_files
                .into_iter()
                .filter_map(|(name_crc32, (new_crc32, path))| {
                    if path == hashes_file {
                        return None;
                    }

                    match hashes.get(&name_crc32) {
                        Some(old_crc32) if *old_crc32 == new_crc32 || path == hashes_file => None,
                        _ => Some(path),
                    }
                })
                .collect();

            println!(
                "{} found {} modified files in input folder, {} files were untoched so we skip them",
                "[+]".green(),
                filterd_files.len(),
                all_files_len - filterd_files.len(),
            );

            filterd_files
        } else {
            println!();
            files
        };

        if files.is_empty() && !self.generate_anyway {
            anyhow::bail!("no modified file found, so there is nothing to import. aborting")
        }

        println!("{} updating archive entries", "[+]".green());

        let mut updated = false;
        for mut entry in archive.files_mut() {
            if !files.contains(&entry.path) {
                continue;
            }

            entry.update(UpdateKind::File(entry.path.clone()));
            updated = true;
        }

        if !updated && !self.generate_anyway {
            anyhow::bail!("nothing in the archive updated. aborting")
        } else if self.generate_anyway {
            println!(
                "{} updated nothing in the archive, rebuilding anyway",
                "[+]".green()
            );
        }

        println!(
            "{} starting the process of creating a new hvp archive",
            "[+]".green()
        );

        // this is hacky but it'll work
        std::env::set_current_dir(org_working_dir)
            .context("failed to change working directory to original base path")?;

        let mut writer = BufWriter::new(
            File::create(output).context("failed to create output hvp archive file")?,
        );

        std::env::set_current_dir(&self.input_folder)
            .context("failed to change working directory to output path")?;

        let pb = utils::progress_bar(archive.metadata().file_count as _);
        let progress = RebuildProgressCli(pb.clone());

        archive
            .rebuild(&mut writer, progress)
            .context("failed to rebuild the archive")?;

        pb.finish_with_message(
            "rebuild finished"
                .if_supports_color(owo_colors::Stream::Stdout, |t| t.green())
                .to_string(),
        );

        writer.flush().context("failed to flush writer")?;

        println!("{} rebuild finished", "[+]".green());

        Ok(())
    }
}

struct RebuildProgressCli(ProgressBar);

impl RebuildProgress for RebuildProgressCli {
    fn inc(&self, message: Option<String>) {
        self.0.inc(1);
        if let Some(msg) = message {
            self.0.set_message(msg);
        }
    }

    fn inc_n(&self, n: usize, message: Option<String>) {
        self.0.inc(n as _);
        if let Some(msg) = message {
            self.0.set_message(msg);
        }
    }
}
