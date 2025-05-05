use std::{
    ffi::OsStr,
    fs::File,
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use binrw::{BinRead, BinWrite, Endian, io::BufReader};
use clap::{Parser, Subcommand, ValueEnum, ValueHint};
use flate2::{Compression, FlushCompress};
use structures::archive::obscure1;

mod utils;

/// Obscure hvp tool
#[derive(clap::Parser)]
#[command(
    name = "Obscure hvp tool",
    author,
    version,
    arg_required_else_help = true
)]
struct Commands {
    #[command(subcommand)]
    operation: Operation,
    /// What game is the archive from
    #[arg(short = 'g', default_value_t = Game::Obscure1, value_enum, global = true)]
    game: Game,
}

#[derive(Subcommand)]
enum Operation {
    #[cfg(feature = "dump")]
    /// dump hvp archive TOC as json
    #[command(arg_required_else_help = true)]
    Dump {
        /// path to input hvp archive
        #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
        input: PathBuf,
        /// output json file, if empty a json file with the same name of input hvp will be created
        output: Option<PathBuf>,
    },
    /// extract files from hvp archive
    #[command(arg_required_else_help = true)]
    Extract {
        /// path to input hvp archive
        #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
        input: PathBuf,
        /// output folder, if empty a folder with the same name as input will be used
        #[arg(value_hint = ValueHint::DirPath)]
        output_folder: Option<PathBuf>,
        /// skip checksum validatation
        #[arg(long, short = 's', default_value_t = false, required = false)]
        skip_checksum_validatation: bool,
    },
    /// create a new hvp archive based on extracted data and original archive
    #[command(arg_required_else_help = true)]
    Create {
        /// path to input hvp archive
        #[arg(value_hint = ValueHint::FilePath, value_parser = utils::is_file)]
        input_hvp: PathBuf,
        /// path to folder of exported data
        #[arg(value_hint = ValueHint::DirPath, value_parser = utils::is_dir)]
        input_folder: PathBuf,
        /// output file, if empty a new file with the same name of input hvp will be created (+ new)
        output: Option<PathBuf>,
        /// skip compression of the files
        #[arg(long, short = 'c', default_value_t = false, required = false)]
        skip_compression: bool,
    },
}

#[derive(ValueEnum, Copy, Clone, Debug, Default)]
enum Game {
    /// auto detect the game based on input hvp
    #[default]
    Auto,
    /// the obscure game
    Obscure1,
}

fn main() -> anyhow::Result<()> {
    let cmd = Commands::parse();

    // for now we only support obscure 1 so we don't check for game input

    match cmd.operation {
        #[cfg(feature = "dump")]
        Operation::Dump { input, output } => {
            let output = output.unwrap_or_else(|| input.with_extension("json"));
            let file = File::open(input).context("failed to open input hvp file")?;
            let mut reader = BufReader::new(file);

            // load the archive
            let archive = obscure1::HvpArchive::read_options(&mut reader, Endian::Big, ())
                .context("failed to load hvp archive")?;

            println!("[+] Loaded hvp archive");
            println!(" - root files/dirs: {}", archive.header.root_count);
            println!(" - all files/folders: {}", archive.header.all_count);
            println!(" - files: {}", archive.header.file_count);

            let outfile = File::create(&output).context("failed to create output json file")?;
            let writer = BufWriter::new(outfile);

            serde_json::to_writer_pretty(writer, &archive)
                .context("failed to dump archive information as json")?;

            println!(
                "[D] dump process finished and json file saved as '{}'",
                output.display()
            );
        }
        Operation::Extract {
            input,
            output_folder,
            skip_checksum_validatation,
        } => {
            let output = output_folder.unwrap_or_else(|| input.with_extension(""));
            let file = File::open(input).context("failed to open input hvp file")?;
            let mut reader = BufReader::new(file);

            if !output.is_dir() {
                println!("[!] Creating output folder");
                std::fs::create_dir(&output).context("failed to create output folder")?;
            }

            // load the archive
            let archive = obscure1::HvpArchive::read_options(&mut reader, Endian::Big, ())
                .context("failed to load hvp archive")?;

            println!("[+] Loaded hvp archive");
            println!(" - root files/dirs: {}", archive.header.root_count);
            println!(" - all files/folders: {}", archive.header.all_count);
            println!(" - files: {}", archive.header.file_count);

            // starting the export process
            let mut extractor = Extractor {
                reader,
                output: output.clone(),
                skip_checksum_validatation,
            };

            for entry in archive.entries {
                match entry.kind {
                    obscure1::EntryKind::Dir(entry) => extractor.dir(entry, None)?,
                    obscure1::EntryKind::File(entry) => extractor.file(entry, None)?,
                }
            }

            println!(
                "[D] export process finished and all files saved in '{}' folder",
                output.display()
            );
        }
        Operation::Create {
            input_hvp,
            input_folder,
            output,
            skip_compression,
        } => {
            let output = output.unwrap_or_else(|| {
                input_hvp.with_extension(
                    input_hvp
                        .extension()
                        .and_then(OsStr::to_str)
                        .map(|e| format!("new.{e}"))
                        .unwrap_or("new".to_owned()),
                )
            });
            let file = File::open(input_hvp).context("failed to open input hvp file")?;
            let mut reader = BufReader::new(file);

            let file_list = utils::list_files(&input_folder, true);

            if file_list.is_empty() {
                anyhow::bail!("there's no file in input folder");
            }

            println!("[+] found {} files in input folder", file_list.len());

            // load the archive
            let mut archive = obscure1::HvpArchive::read_options(&mut reader, Endian::Big, ())
                .context("failed to load hvp archive")?;

            // store the position after reading archive so we can resize output file based on it
            let after_read_archive_header_pos = reader
                .stream_position()
                .context("failed to get reader position")?;

            println!("[+] Loaded hvp archive");
            println!(" - root files/dirs: {}", archive.header.root_count);
            println!(" - all files/folders: {}", archive.header.all_count);
            println!(" - files: {}", archive.header.file_count);

            let mut out_file = File::create(&output).context("failed to create output file")?;

            // seek after the header
            out_file
                .seek(SeekFrom::Start(after_read_archive_header_pos))
                .context("failed to seek after in reader")?;

            let writer = BufWriter::new(out_file);

            let mut creator = Creator {
                reader,
                writer,
                file_list,
                input_folder,
                skip_compression,
            };

            for entry in archive.entries.iter_mut() {
                match &mut entry.kind {
                    obscure1::EntryKind::Dir(entry) => creator.dir(entry, None)?,
                    obscure1::EntryKind::File(entry) => creator.file(entry, None)?,
                }
            }

            // seek back to start and write the header
            creator
                .writer
                .seek(SeekFrom::Start(0))
                .context("failed to seek to start of writer")?;

            // update checksums
            archive
                .update_checksums(Endian::Big)
                .context("failed to update archive checksums")?;

            archive
                .write_options(&mut creator.writer, Endian::Big, ())
                .context("failed to write archive header to writer")?;

            creator.writer.flush().context("failed to flush writer")?;

            println!(
                "[D] create process successfully finished and a new archive saved at '{}'",
                output.display()
            );
        }
    }

    Ok(())
}

struct Extractor {
    reader: BufReader<File>,
    output: PathBuf,
    skip_checksum_validatation: bool,
}

impl Extractor {
    fn dir(&mut self, entry: obscure1::DirEntry, parent: Option<&Path>) -> anyhow::Result<()> {
        let path = match parent {
            Some(parent) => parent.join(entry.name),
            None => PathBuf::from(entry.name),
        };

        let full_path = self.output.join(&path);
        if !full_path.is_dir() {
            std::fs::create_dir(&full_path).context("failed to create one of output dir")?;
        }

        for entry in entry.entries {
            match entry.kind {
                obscure1::EntryKind::Dir(entry) => self.dir(entry, Some(&path))?,
                obscure1::EntryKind::File(entry) => self.file(entry, Some(&path))?,
            }
        }

        Ok(())
    }

    fn file(&mut self, entry: obscure1::FileEntry, parent: Option<&Path>) -> anyhow::Result<()> {
        let path = match parent {
            Some(parent) => parent.join(&entry.name),
            None => PathBuf::from(&entry.name),
        };

        if entry.uncompressed_size == 0 {
            println!("[+] Skipping '{}' because it's empty", path.display());
            return Ok(());
        }

        self.reader
            .seek(SeekFrom::Start(entry.offset as u64))
            .context("failed to seek to file entry offset")?;

        let mut buf = vec![0_u8; entry.compressed_size as usize];
        self.reader
            .read_exact(&mut buf)
            .context("failed to read file entry data")?;

        if !self.skip_checksum_validatation {
            let data_checksum = obscure1::bytes_sum(&buf);
            if entry.checksum != data_checksum {
                anyhow::bail!(
                    "checksum of file {} doesn't match with its data ({} != {})",
                    entry.name,
                    entry.checksum,
                    data_checksum
                );
            }
        }

        if entry.is_compressed {
            let mut decompressed_buf = vec![0_u8; entry.uncompressed_size as usize];

            flate2::Decompress::new(true)
                .decompress(&buf, &mut decompressed_buf, flate2::FlushDecompress::Finish)
                .context("failed to decompress file entry data")?;

            buf = decompressed_buf;
        }

        let out_path = self.output.join(&path);
        std::fs::write(out_path, buf).context("failed to write file entry data to disk")?;
        println!("[+] Extracted '{}'", path.display());

        Ok(())
    }
}

// that's me d:
struct Creator {
    reader: BufReader<File>,
    writer: BufWriter<File>,
    file_list: Vec<PathBuf>,
    input_folder: PathBuf,
    skip_compression: bool,
}

impl Creator {
    fn dir(&mut self, entry: &mut obscure1::DirEntry, parent: Option<&Path>) -> anyhow::Result<()> {
        let path = match parent {
            Some(parent) => parent.join(&entry.name),
            None => PathBuf::from(&entry.name),
        };

        for entry in entry.entries.iter_mut() {
            match &mut entry.kind {
                obscure1::EntryKind::Dir(entry) => self.dir(entry, Some(&path))?,
                obscure1::EntryKind::File(entry) => self.file(entry, Some(&path))?,
            }
        }

        Ok(())
    }

    fn file(
        &mut self,
        entry: &mut obscure1::FileEntry,
        parent: Option<&Path>,
    ) -> anyhow::Result<()> {
        let path = match parent {
            Some(parent) => parent.join(&entry.name),
            None => PathBuf::from(&entry.name),
        };

        if self.file_list.contains(&path) {
            // read from input folder
            let full_path = self.input_folder.join(&path);
            let mut buf =
                std::fs::read(full_path).context("failed to read file from input folder")?;

            entry.compressed_size = buf.len() as u32;
            entry.uncompressed_size = buf.len() as u32;

            if self.skip_compression && entry.is_compressed {
                entry.is_compressed = false;
            }

            if entry.is_compressed {
                let mut compressed_buf = Vec::with_capacity(deflate_bound(buf.len()));
                flate2::Compress::new(Compression::best(), true)
                    .compress_vec(&buf, &mut compressed_buf, FlushCompress::Finish)
                    .context("failed to compress file using zlib")?;

                compressed_buf.shrink_to_fit();
                entry.compressed_size = compressed_buf.len() as u32;
                buf = compressed_buf;
            }

            entry.offset = self
                .writer
                .stream_position()
                .context("failed to get writer position")? as u32;

            entry.checksum = obscure1::bytes_sum(&buf);

            self.writer
                .write_all(&buf)
                .context("failed to copy buffer from original hvp archive to the new one")?;

            println!("[+] Added '{}' from input folder", path.display());
        } else {
            // read from archive

            if entry.uncompressed_size == 0 {
                println!("[+] Skipping '{}' because it's empty", path.display());
                return Ok(());
            }

            self.reader
                .seek(SeekFrom::Start(entry.offset as u64))
                .context("failed to seek to file entry offset")?;

            let mut buf = vec![0_u8; entry.compressed_size as usize];
            self.reader
                .read_exact(&mut buf)
                .context("failed to read file entry data")?;

            entry.offset = self
                .writer
                .stream_position()
                .context("failed to get writer position")? as u32;

            self.writer
                .write_all(&buf)
                .context("failed to copy buffer from original hvp archive to the new one")?;

            // no need to change compressed or uncompressed sizes

            println!("[+] Added '{}' from original archive", path.display());
        }

        Ok(())
    }
}

fn deflate_bound(source_len: usize) -> usize {
    source_len + (source_len >> 12) + (source_len >> 14) + 11 - ((source_len >> 1) & 1)
}
