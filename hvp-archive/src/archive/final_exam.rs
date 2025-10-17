use std::io::{Read, Write};
use std::ops::Range;

use binrw::Endian;

use super::Metadata;
use super::entry::{CompressionInfo, CompressionType, DirEntry, Entry, FileEntry};
use super::error::RebuildError;
use super::rebuild_progress::RebuildProgress;
use crate::Game;
use crate::provider::ArchiveProvider;
use crate::structures::{checksum, final_exam};

/// map the entries and return them plus the number of files
pub fn map_entries<'p>(
    provider: &'p ArchiveProvider,
    entries: &[final_exam::Entry],
    endian: Endian,
    names: &final_exam::Names,
) -> (Vec<Entry<'p>>, Metadata) {
    // we ignore the root dir, because it really don't serve any purpose except adding one layer of nesting
    // we can manually add it when we are writing the entries back
    let root_count = match &entries[0] {
        final_exam::Entry {
            name_crc32: 0,
            kind:
                final_exam::EntryKind::Directory(final_exam::DirEntry {
                    index: 1, count, ..
                }),
        } => *count as usize,
        _ => unreachable!("found a hvp without valid root entry"),
    };

    let mut process = Process {
        provider,
        entries,
        endian,
        names,
        metadata: Metadata {
            dir_count: 0,
            file_count: 0,
            game: Game::FinalExam,
        },
    };

    let entries = entries[1..1 + root_count]
        .iter()
        .map(|entry| process.process_entry(entry))
        .collect();

    (entries, process.metadata)
}

/// a helper for processing final exam entries
struct Process<'p, 'e, 'n> {
    provider: &'p ArchiveProvider,
    entries: &'e [final_exam::Entry],
    endian: Endian,
    names: &'n final_exam::Names,
    metadata: Metadata,
}

impl<'p> Process<'p, '_, '_> {
    #[inline]
    fn process_entry(&mut self, entry: &final_exam::Entry) -> Entry<'p> {
        match &entry.kind {
            final_exam::EntryKind::File(file) => self.process_file(file, false),
            final_exam::EntryKind::FileCompressed(file) => self.process_file(file, true),
            final_exam::EntryKind::Directory(dir) => self.process_dir(dir, dir.entries_range()),
        }
    }

    fn process_file(&mut self, entry: &final_exam::FileEntry, is_compressed: bool) -> Entry<'p> {
        let name = self.names.get_name_by_offset(entry.name_offset).to_owned();

        self.metadata.file_count += 1;

        Entry::File(FileEntry {
            name,
            compression_info: is_compressed.then_some(CompressionInfo {
                uncompressed_size: entry.uncompressed_size,
                compression_type: CompressionType::Lzo,
            }),
            checksum: entry.checksum,
            endian: self.endian,
            raw_bytes: self
                .provider
                .get_bytes(entry.offset as _, entry.compressed_size as _),
            update: None,
        })
    }

    fn process_dir(&mut self, entry: &final_exam::DirEntry, range: Range<usize>) -> Entry<'p> {
        let name = self.names.get_name_by_offset(entry.name_offset).to_owned();

        let mut dir = DirEntry {
            name,
            entries: Vec::with_capacity(entry.count as usize),
        };

        self.metadata.dir_count += 1;

        for e in &self.entries[range] {
            match &e.kind {
                final_exam::EntryKind::File(file_entry) => {
                    dir.entries.push(self.process_file(file_entry, false))
                }
                final_exam::EntryKind::FileCompressed(file_entry) => {
                    dir.entries.push(self.process_file(file_entry, true))
                }
                final_exam::EntryKind::Directory(dir_entry) => dir
                    .entries
                    .push(self.process_dir(dir_entry, dir_entry.entries_range())),
            }
        }

        Entry::Dir(dir)
    }
}

/// update the archive entries based on the mapped entries
pub fn update_entries<W: Write, P: RebuildProgress>(
    writer: &mut W,
    offset: u32,
    skip_compression: bool,
    mut archive: final_exam::HvpArchive,
    entries: &[Entry],
    names: &final_exam::Names,
    progress: P,
) -> Result<final_exam::HvpArchive, RebuildError> {
    // we ignore the root dir, because it really don't serve any purpose except adding one layer of nesting
    // we can manually add it when we are writing the entries back
    let root_count = match &archive.entries[0] {
        final_exam::Entry {
            name_crc32: 0,
            kind:
                final_exam::EntryKind::Directory(final_exam::DirEntry {
                    index: 1, count, ..
                }),
        } => *count as usize,
        _ => unreachable!("found a hvp without valid root entry"),
    };

    let mut updater = Updater {
        writer,
        progress,
        offset,
        skip_compression,
        names,
        endian: archive.endian(),
    };

    updater.caculate_and_apply_padding()?;

    let mut entries_iter = entries.iter();
    for o_entry_idx in 1..1 + root_count {
        let Some(u_entry) = entries_iter.next() else {
            unreachable!("number of parsed entries doesn't match with original entries");
        };

        updater.process_entry(o_entry_idx, u_entry, &mut archive.entries)?;
    }

    Ok(archive)
}

/// a helper for making the updating easier
pub struct Updater<'a, 'n, W: Write, P: RebuildProgress> {
    writer: &'a mut W,
    progress: P,
    offset: u32,
    skip_compression: bool,
    names: &'n final_exam::Names,
    // BigEndian version have 32 padding
    endian: Endian,
}

impl<W: Write, P: RebuildProgress> Updater<'_, '_, W, P> {
    fn process_entry(
        &mut self,
        o_entry_idx: usize,
        u_entry: &Entry,
        entries: &mut [final_exam::Entry],
    ) -> Result<(), RebuildError> {
        // at points like this I say to myself, wtf is rust about...
        // not being able to have multiple mutable borrow to same value made me
        // to write the code like this... and one useless clone as well...
        // this sucks!
        if let (
            final_exam::EntryKind::FileCompressed(o_entry) | final_exam::EntryKind::File(o_entry),
            Entry::File(u_entry),
        ) = (&mut entries[o_entry_idx].kind, u_entry)
        {
            self.process_file(o_entry, u_entry)?;
            self.caculate_and_apply_padding()?;

            Ok(())
        } else if let (final_exam::EntryKind::Directory(o_entry), Entry::Dir(u_entry)) =
            (&entries[o_entry_idx].kind, u_entry)
        {
            self.process_dir(u_entry, o_entry.entries_range(), entries)
        } else {
            unreachable!()
        }
    }

    fn process_file(
        &mut self,
        o_entry: &mut final_exam::FileEntry,
        u_entry: &FileEntry,
    ) -> Result<(), RebuildError> {
        assert_eq!(
            o_entry.checksum, u_entry.checksum,
            "checksum original entry and updated entry doesn't match"
        );

        let name = self
            .names
            .get_name_by_offset(o_entry.name_offset)
            .to_owned();

        if o_entry.uncompressed_size == 0 {
            self.progress.inc(Some(format!("(skp) {name}")));

            return Ok(());
        }

        o_entry.offset = self.offset;

        let Some(update) = &u_entry.update else {
            self.progress.inc(Some(format!("(src) {name}")));
            self.writer.write_all(u_entry.raw_bytes)?;
            self.offset += u_entry.raw_bytes.len() as u32;
            return Ok(());
        };

        let bytes = update.to_bytes()?;

        self.progress.inc(Some(format!("(upd) {name}")));

        if self.skip_compression || !u_entry.is_compressed() {
            self.writer.write_all(&bytes)?;
            self.offset += bytes.len() as u32;
            o_entry.compressed_size = bytes.len() as _;
            o_entry.uncompressed_size = bytes.len() as _;
            o_entry.checksum = checksum::bytes_sum(&bytes, self.endian);
            return Ok(());
        }

        let compressed_bytes = lzo1x::compress(&bytes, lzo1x::CompressLevel::new(12));

        self.writer.write_all(&compressed_bytes)?;
        self.offset += compressed_bytes.len() as u32;
        o_entry.compressed_size = compressed_bytes.len() as _;
        o_entry.uncompressed_size = bytes.len() as _;
        o_entry.checksum = checksum::bytes_sum(&compressed_bytes, self.endian);

        Ok(())
    }

    fn process_dir(
        &mut self,
        u_entry: &DirEntry,
        range: Range<usize>,
        entries: &mut [final_exam::Entry],
    ) -> Result<(), RebuildError> {
        let mut entries_iter = u_entry.entries.iter();
        for o_entry_idx in range {
            let Some(u_entry) = entries_iter.next() else {
                unreachable!("number of parsed entries doesn't match with original entries");
            };

            self.process_entry(o_entry_idx, u_entry, entries)?;
        }

        Ok(())
    }

    #[inline]
    fn caculate_and_apply_padding(&mut self) -> std::io::Result<()> {
        if self.offset % 4 != 0 {
            let last_padding = 4 - (self.offset % 4);
            std::io::copy(&mut std::io::repeat(0).take(last_padding as _), self.writer)?;
            self.offset += last_padding;
        }

        Ok(())
    }
}
