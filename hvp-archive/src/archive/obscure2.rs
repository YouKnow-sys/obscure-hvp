use std::io::Write;
use std::ops::Range;

use lzokay_native::Dict;

use super::Metadata;
use super::entry::{CompressionInfo, CompressionType, DirEntry, Entry, FileEntry};
use super::error::RebuildError;
use super::rebuild_progress::RebuildProgress;
use crate::Game;
use crate::provider::ArchiveProvider;
use crate::structures::{checksum, obscure2};

/// map the entries and return them plus the number of files
pub fn map_entries<'p>(
    provider: &'p ArchiveProvider,
    entries: &[obscure2::Entry],
    name_map: &Obscure2NameMap,
) -> (Vec<Entry<'p>>, Metadata) {
    // we ignore the root dir, because it really don't serve any purpose except adding one layer of nesting
    // we can manually add it when we are writing the entries back
    let root_count = match &entries[0] {
        obscure2::Entry {
            name_crc32: 0,
            kind:
                obscure2::EntryKind::Directory(obscure2::DirEntry {
                    index: 1, count, ..
                }),
        } => *count as usize,
        _ => unreachable!("found a hvp without valid root entry"),
    };

    let mut process = Process {
        provider,
        entries,
        name_map,
        metadata: Metadata {
            dir_count: 0,
            file_count: 0,
            game: Game::Obscure2,
        },
    };

    let entries = entries[1..1 + root_count]
        .iter()
        .map(|entry| process.process_entry(entry))
        .collect();

    (entries, process.metadata)
}

/// a helper for processing obscure 2 entries
struct Process<'p, 'e, 'n> {
    provider: &'p ArchiveProvider,
    entries: &'e [obscure2::Entry],
    name_map: &'n Obscure2NameMap,
    metadata: Metadata,
}

impl<'p> Process<'p, '_, '_> {
    #[inline]
    fn process_entry(&mut self, entry: &obscure2::Entry) -> Entry<'p> {
        match &entry.kind {
            obscure2::EntryKind::File(file) => self.process_file(file, entry.name_crc32, false),
            obscure2::EntryKind::FileCompressed(file) => {
                self.process_file(file, entry.name_crc32, true)
            }
            obscure2::EntryKind::Directory(dir) => {
                self.process_dir(entry.name_crc32, dir, dir.entries_range())
            }
        }
    }

    fn process_file(
        &mut self,
        entry: &obscure2::FileEntry,
        name_crc32: u32,
        is_compressed: bool,
    ) -> Entry<'p> {
        let name = self
            .name_map
            .get_name(name_crc32)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                log::warn!("unknown obscure2 file hash {name_crc32}");
                format!("unk_file_{name_crc32}.dat")
            });

        self.metadata.file_count += 1;

        Entry::File(FileEntry {
            name,
            compression_info: is_compressed.then_some(CompressionInfo {
                uncompressed_size: entry.uncompressed_size,
                compression_type: CompressionType::Lzo,
            }),
            checksum: entry.checksum,
            raw_bytes: self
                .provider
                .get_bytes(entry.offset as _, entry.compressed_size as _),
            update: None,
        })
    }

    fn process_dir(
        &mut self,
        name_crc32: u32,
        entry: &obscure2::DirEntry,
        range: Range<usize>,
    ) -> Entry<'p> {
        let name = self
            .name_map
            .get_name(name_crc32)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                log::warn!("unknown obscure2 dir hash {name_crc32}");
                format!("unk_folder_{name_crc32}")
            });

        let mut dir = DirEntry {
            name,
            entries: Vec::with_capacity(entry.count as usize),
        };

        self.metadata.dir_count += 1;

        for e in &self.entries[range] {
            match &e.kind {
                obscure2::EntryKind::File(file_entry) => {
                    dir.entries
                        .push(self.process_file(file_entry, e.name_crc32, false))
                }
                obscure2::EntryKind::FileCompressed(file_entry) => dir
                    .entries
                    .push(self.process_file(file_entry, e.name_crc32, true)),
                obscure2::EntryKind::Directory(dir_entry) => dir.entries.push(self.process_dir(
                    e.name_crc32,
                    dir_entry,
                    dir_entry.entries_range(),
                )),
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
    mut archive: obscure2::HvpArchive,
    entries: &[Entry],
    name_map: &Obscure2NameMap,
    progress: P,
) -> Result<obscure2::HvpArchive, RebuildError> {
    // we ignore the root dir, because it really don't serve any purpose except adding one layer of nesting
    // we can manually add it when we are writing the entries back
    let root_count = match &archive.entries()[0] {
        obscure2::Entry {
            name_crc32: 0,
            kind:
                obscure2::EntryKind::Directory(obscure2::DirEntry {
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
        name_map,
        compress_dict: Dict::new(),
    };

    let mut entries_iter = entries.iter();
    for o_entry_idx in 1..1 + root_count {
        let Some(u_entry) = entries_iter.next() else {
            unreachable!("number of parsed entries doesn't match with original entries");
        };

        updater.process_entry(o_entry_idx, u_entry, archive.entries_mut())?;
    }

    archive.update_checksums().unwrap();

    Ok(archive)
}

/// a helper for making the updating easier
pub struct Updater<'a, 'n, W: Write, P: RebuildProgress> {
    writer: &'a mut W,
    progress: P,
    offset: u32,
    skip_compression: bool,
    name_map: &'n Obscure2NameMap,
    compress_dict: Dict,
}

impl<W: Write, P: RebuildProgress> Updater<'_, '_, W, P> {
    fn process_entry(
        &mut self,
        o_entry_idx: usize,
        u_entry: &Entry,
        entries: &mut [obscure2::Entry],
    ) -> Result<(), RebuildError> {
        // at points like this I say to myself, wtf is rust about...
        // not being able to have multiple mutable borrow to same value made me
        // to write the code like this... and onee useless clone as well...
        // this sucks!
        if let (
            obscure2::EntryKind::FileCompressed(o_entry) | obscure2::EntryKind::File(o_entry),
            Entry::File(u_entry),
        ) = (&mut entries[o_entry_idx].kind, u_entry)
        {
            self.process_file(entries[o_entry_idx].name_crc32, o_entry, u_entry)
        } else if let (obscure2::EntryKind::Directory(o_entry), Entry::Dir(u_entry)) =
            (&entries[o_entry_idx].kind, u_entry)
        {
            self.process_dir(u_entry, o_entry.entries_range(), entries)
        } else {
            unreachable!()
        }
    }

    fn process_file(
        &mut self,
        name_crc32: u32,
        o_entry: &mut obscure2::FileEntry,
        u_entry: &FileEntry,
    ) -> Result<(), RebuildError> {
        assert_eq!(
            o_entry.checksum, u_entry.checksum,
            "checksum original entry and updated entry doesn't match"
        );

        let name = self
            .name_map
            .get_name(name_crc32)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("unk_file_{name_crc32}.dat"));

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
            o_entry.checksum = checksum::bytes_sum(&bytes);
            return Ok(());
        }

        let compressed_bytes = lzokay_native::compress_with_dict(&bytes, &mut self.compress_dict)?;

        self.writer.write_all(&compressed_bytes)?;
        self.offset += compressed_bytes.len() as u32;
        o_entry.compressed_size = compressed_bytes.len() as _;
        o_entry.uncompressed_size = bytes.len() as _;
        o_entry.checksum = checksum::bytes_sum(&compressed_bytes);

        Ok(())
    }

    fn process_dir(
        &mut self,
        u_entry: &DirEntry,
        range: Range<usize>,
        entries: &mut [obscure2::Entry],
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
}

/// obscure 2 name map
#[derive(Debug, Default)]
pub struct Obscure2NameMap(ahash::HashMap<u32, String>);

impl Obscure2NameMap {
    pub fn new<I>(names: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let map = names
            .into_iter()
            .map(|n| {
                let name = n.as_ref().to_owned();
                let crc32 = get_name_crc32(&name);

                (crc32, name)
            })
            .collect();

        Self(map)
    }

    /// get a name using crc32 of it
    pub fn get_name(&self, crc32: u32) -> Option<&str> {
        self.0.get(&crc32).map(String::as_str)
    }

    pub fn get_crc32_from_name(&self, name: &str) -> u32 {
        let crc32 = get_name_crc32(name);

        debug_assert!(
            self.0.is_empty() || self.0.contains_key(&crc32),
            "can't find input name crc32 in the namemap"
        );

        crc32
    }
}

#[inline]
fn get_name_crc32(name: &str) -> u32 {
    if name.contains('é') {
        // we do this because of windows-1250 encoding
        // as far as I checked names may only have 'é', so
        // no need to use crates like `encoding_rs`
        let bytes: Vec<u8> = name
            .chars()
            .map(|ch| match ch {
                'é' => 0xE9,
                c => {
                    assert!(
                        c.is_ascii(),
                        "found a character that isn't ascii when generating crc32 of name"
                    );

                    c as u8
                }
            })
            .collect();

        crc32fast::hash(&bytes)
    } else {
        crc32fast::hash(name.as_bytes())
    }
}
