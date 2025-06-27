use std::io::Write;

use binrw::Endian;
use flate2::{Compress, Compression, FlushCompress};

use super::Metadata;
use super::entry::{CompressionInfo, CompressionType, DirEntry, Entry, FileEntry};
use super::error::RebuildError;
use super::rebuild_progress::RebuildProgress;
use crate::Game;
use crate::provider::ArchiveProvider;
use crate::structures::{checksum, obscure1};

/// map the entries and return them plus the number of files
pub fn map_entries<'p>(
    provider: &'p ArchiveProvider,
    entries: &[obscure1::Entry],
) -> (Vec<Entry<'p>>, Metadata) {
    let mut process = Process {
        provider,
        metadata: Metadata {
            dir_count: 0,
            file_count: 0,
            game: Game::Obscure1,
        },
    };

    let entries = entries
        .iter()
        .map(|entry| process.process_entry(entry))
        .collect();

    (entries, process.metadata)
}

/// a helper for processing obscure 1 entries
struct Process<'p> {
    provider: &'p ArchiveProvider,
    metadata: Metadata,
}

impl<'p> Process<'p> {
    #[inline]
    fn process_entry(&mut self, entry: &obscure1::Entry) -> Entry<'p> {
        match &entry.kind {
            obscure1::EntryKind::Dir(entry) => self.process_dir(entry),
            obscure1::EntryKind::File(entry) => self.process_file(entry),
        }
    }

    fn process_file(&mut self, entry: &obscure1::FileEntry) -> Entry<'p> {
        let raw_bytes = if entry.uncompressed_size == 0 {
            self.provider.get_empty_bytes()
        } else {
            self.provider
                .get_bytes(entry.offset as _, entry.compressed_size as _)
        };

        self.metadata.file_count += 1;

        Entry::File(FileEntry {
            name: entry.name.clone(),
            compression_info: entry.is_compressed.then_some(CompressionInfo {
                uncompressed_size: entry.uncompressed_size,
                compression_type: CompressionType::Zlib,
            }),
            checksum: entry.checksum,
            endian: Endian::Little,
            raw_bytes,
            update: None,
        })
    }

    fn process_dir(&mut self, entry: &obscure1::DirEntry) -> Entry<'p> {
        self.metadata.dir_count += 1;

        let entries = entry
            .entries
            .iter()
            .map(|entry| self.process_entry(entry))
            .collect();

        Entry::Dir(DirEntry {
            name: entry.name.clone(),
            entries,
        })
    }
}

/// update the archive entries based on the mapped entries
pub fn update_entries<W: Write, P: RebuildProgress>(
    writer: &mut W,
    offset: u32,
    skip_compression: bool,
    mut archive: obscure1::HvpArchive,
    entries: &[Entry],
    progress: P,
) -> Result<obscure1::HvpArchive, RebuildError> {
    assert_eq!(
        archive.entries.len(),
        entries.len(),
        "size of entries doesn't match"
    );

    let mut updater = Updater {
        writer,
        progress,
        offset,
        skip_compression,
    };

    for (o, u) in archive.entries.iter_mut().zip(entries) {
        match (&mut o.kind, u) {
            (obscure1::EntryKind::Dir(o_entry), Entry::Dir(u_entry)) => {
                updater.process_dir(o_entry, u_entry)?;
            }
            (obscure1::EntryKind::File(o_entry), Entry::File(u_entry)) => {
                updater.process_file(o_entry, u_entry)?;
            }
            _ => unreachable!(),
        }
    }

    Ok(archive)
}

/// a helper for making the updating easier
struct Updater<'a, W: Write, P: RebuildProgress> {
    writer: &'a mut W,
    progress: P,
    offset: u32,
    skip_compression: bool,
}

impl<W: Write, P: RebuildProgress> Updater<'_, W, P> {
    fn process_file(
        &mut self,
        o_entry: &mut obscure1::FileEntry,
        u_entry: &FileEntry,
    ) -> Result<(), RebuildError> {
        if o_entry.uncompressed_size == 0 {
            self.progress.inc(Some(format!("(skp) {}", o_entry.name)));
            return Ok(());
        }

        o_entry.offset = self.offset;

        let Some(update) = &u_entry.update else {
            self.progress.inc(Some(format!("(src) {}", o_entry.name)));
            self.writer.write_all(u_entry.raw_bytes)?;
            self.offset += u_entry.raw_bytes.len() as u32;
            return Ok(());
        };

        let bytes = update.to_bytes()?;

        self.progress.inc(Some(format!("(upd) {}", o_entry.name)));

        if self.skip_compression || !o_entry.is_compressed {
            self.writer.write_all(&bytes)?;
            self.offset += bytes.len() as u32;
            o_entry.compressed_size = bytes.len() as _;
            o_entry.uncompressed_size = bytes.len() as _;
            o_entry.is_compressed = false;
            o_entry.checksum = checksum::bytes_sum(&bytes, Endian::Little);
            return Ok(());
        }

        let mut compressed_buf = Vec::with_capacity(deflate_bound(bytes.len()));
        Compress::new(Compression::best(), true).compress_vec(
            &bytes,
            &mut compressed_buf,
            FlushCompress::Finish,
        )?;

        self.writer.write_all(&compressed_buf)?;
        self.offset += compressed_buf.len() as u32;
        o_entry.compressed_size = compressed_buf.len() as _;
        o_entry.uncompressed_size = bytes.len() as _;
        o_entry.checksum = checksum::bytes_sum(&compressed_buf, Endian::Little);

        Ok(())
    }

    fn process_dir(
        &mut self,
        o_entry: &mut obscure1::DirEntry,
        u_entry: &DirEntry,
    ) -> Result<(), RebuildError> {
        for (o, u) in o_entry.entries.iter_mut().zip(&u_entry.entries) {
            match (&mut o.kind, u) {
                (obscure1::EntryKind::Dir(o_entry), Entry::Dir(u_entry)) => {
                    self.process_dir(o_entry, u_entry)?;
                }
                (obscure1::EntryKind::File(o_entry), Entry::File(u_entry)) => {
                    self.process_file(o_entry, u_entry)?;
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}

fn deflate_bound(source_len: usize) -> usize {
    source_len + (source_len >> 12) + (source_len >> 14) + 11 - ((source_len >> 1) & 1)
}
