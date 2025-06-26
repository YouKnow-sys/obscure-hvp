//! a full abstraction over obscure 1 and 2 hvp archives

use std::{
    fmt::Debug,
    io::{Read, Seek, SeekFrom, Write},
};

use crate::{
    Game,
    provider::{ArchiveProvider, RawArchive},
};

use binrw::BinWrite;

pub use obscure2::Obscure2NameMap;

use entry::Entry;
use error::RebuildError;
use file_helpers::{FileIterator, FileIteratorMut};
use rebuild_progress::RebuildProgress;

pub mod entry;
pub mod error;
pub mod file_helpers;
mod obscure1;
mod obscure2;
pub mod rebuild_progress;

#[derive(Debug, Default)]
pub struct Options {
    pub obscure2_names: Obscure2NameMap,
    pub rebuild_skip_compression: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    pub dir_count: usize,
    pub file_count: usize,
    pub game: Game,
}

/// ## archive abstraction over both obscure 1 and 2
///
/// can manage entries from both games
pub struct Archive<'p> {
    provider: &'p ArchiveProvider,
    entries: Box<[Entry<'p>]>,
    metadata: Metadata,
    pub options: Options,
}

impl<'p> Archive<'p> {
    /// create a new archive with the given provider and default options
    pub fn new(provider: &'p ArchiveProvider) -> Self {
        Self::new_with_options(provider, Options::default())
    }

    /// create a new archive with the given provider and options
    pub fn new_with_options(provider: &'p ArchiveProvider, options: Options) -> Self {
        let (entries, metadata) = match &provider.raw_archive {
            RawArchive::Obscure1(hvp) => obscure1::map_entries(provider, &hvp.entries),
            RawArchive::Obscure2(hvp) => {
                obscure2::map_entries(provider, &hvp.entries, &options.obscure2_names)
            }
        };

        Self {
            provider,
            entries: entries.into_boxed_slice(),
            metadata,
            options,
        }
    }

    /// get a slice of entries
    #[inline(always)]
    pub fn entries(&self) -> &[Entry<'p>] {
        &self.entries
    }

    /// get a mutable slice of entries, using this method you can update entries
    #[inline(always)]
    pub fn entries_mut(&mut self) -> &mut [Entry<'p>] {
        &mut self.entries
    }

    /// return a iterator over files in the archive
    #[inline(always)]
    pub fn files(&self) -> FileIterator<'_, 'p> {
        FileIterator::new(&self.entries, self.metadata.file_count)
    }

    /// return a iterator over files in the archive.
    /// with support of updating
    #[inline(always)]
    pub fn files_mut(&mut self) -> FileIteratorMut<'_, 'p> {
        FileIteratorMut::new(&mut self.entries, self.metadata.file_count)
    }

    /// check whatever checksum of all entries are valid or not.
    pub fn entries_checksum_match(&self) -> bool {
        self.entries.iter().all(|entry| match entry {
            Entry::File(file_entry) => file_entry.checksum_match(),
            Entry::Dir(_) => true,
        })
    }

    /// get the metadata about the current loaded archive
    pub fn metadata(&self) -> Metadata {
        self.metadata
    }

    /// rebuild the archive and write it to the given writer.
    pub fn rebuild<W: Write + Seek, P: RebuildProgress>(
        &self,
        writer: &mut W,
        progress: P,
    ) -> Result<(), RebuildError> {
        let start_pos = writer.stream_position()?;

        // we skip the size of entries, so we can write them back after
        // writing the bytes
        if writer
            .seek(SeekFrom::Current(self.provider.entries_offset as _))
            .is_err()
        {
            // in case of seek failing
            std::io::copy(
                &mut std::io::repeat(0).take(self.provider.entries_offset as _),
                writer,
            )?;
        }

        let offset = writer.stream_position()? as _;

        match &self.provider.raw_archive {
            RawArchive::Obscure1(archive) => {
                let archive = obscure1::update_entries(
                    writer,
                    offset,
                    self.options.rebuild_skip_compression,
                    archive.clone(),
                    &self.entries,
                    progress,
                )?;

                // write the entries back
                writer.seek(SeekFrom::Start(start_pos))?;
                archive.write_be(writer)?;
            }
            RawArchive::Obscure2(archive) => {
                let archive = obscure2::update_entries(
                    writer,
                    offset,
                    self.options.rebuild_skip_compression,
                    archive.clone(),
                    &self.entries,
                    &self.options.obscure2_names,
                    progress,
                )?;

                // write the entries back
                writer.seek(SeekFrom::Start(start_pos))?;
                archive.write_le(writer)?;
            }
        }

        Ok(())
    }
}

impl<'p> Debug for Archive<'p> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let archive_src = match self.provider.raw_archive {
            RawArchive::Obscure1(_) => "obscure1",
            RawArchive::Obscure2(_) => "obscure2",
        };

        f.debug_struct("Archive")
            .field("provider", &archive_src)
            .field("entries", &format!("[Entry; {}]", self.entries.len()))
            .field("metadata", &self.metadata)
            .field("options", &self.options)
            .finish()
    }
}
