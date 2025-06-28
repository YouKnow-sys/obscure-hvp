use std::{
    fs::File,
    io::{self, Seek, SeekFrom},
};

use binrw::{BinRead, io::BufReader};
use memmap2::{Mmap, MmapOptions};

use crate::structures::{obscure1, obscure2};
use crate::{Game, try_detect_game};

/// provider errors
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("unknown archive, failed to autodetect")]
    UnknownArchive,
    #[error("failed to load archive")]
    ArchiveLoadFailed(#[from] binrw::Error),
    #[error("entry offset or size doesn't fit in archive")]
    EntryOffsetOrSizeDoesntFit,
}

/// hold the underlying raw archive
#[cfg(not(feature = "raw_structure"))]
pub(crate) enum RawArchive {
    Obscure1(obscure1::HvpArchive),
    Obscure2(obscure2::HvpArchive),
}

/// hold the underlying raw archive
#[cfg(feature = "raw_structure")]
pub enum RawArchive {
    Obscure1(obscure1::HvpArchive),
    Obscure2(obscure2::HvpArchive),
}

/// archive provider is the main type that load the hvp archives
///
/// it support both obscure 1 and 2 and can also autodetect the game
/// based on archive magic number.
///
/// it also validate the entries to make sure that the loaded archive isn't broken.
pub struct ArchiveProvider {
    pub(crate) raw_archive: RawArchive,
    pub(crate) mmap: Mmap,
    pub(crate) entries_offset: usize,
}

impl ArchiveProvider {
    /// create a new provider from the given file, optionally you can pass the game that the
    /// archive is belong to, if not passed we'll try to autodetect it using [`crate::try_detect_game`].
    pub fn new(file: File, game: Option<Game>) -> Result<Self, ProviderError> {
        let mut reader = BufReader::new(file);

        let game = match game {
            Some(game) => game,
            None => {
                log::debug!("trying to autodetect game based on archive");
                let game = try_detect_game(&mut reader)?.ok_or(ProviderError::UnknownArchive)?;
                log::info!("autodetected game: {game:?}");
                game
            }
        };

        let raw_archive = match game {
            Game::Obscure1 => RawArchive::Obscure1(obscure1::HvpArchive::read_be(&mut reader)?),
            Game::Obscure2 => RawArchive::Obscure2(obscure2::HvpArchive::read(&mut reader)?),
        };

        let entries_offset = reader.stream_position()? as usize;
        log::debug!("entries offest: {entries_offset}");
        let mut file = reader.into_inner();
        file.seek(SeekFrom::Start(0))?;

        let mmap = unsafe { MmapOptions::new().map(&file)? };

        log::info!("validating entries offset and sizes");
        if !validate_entries(&raw_archive, &mmap) {
            return Err(ProviderError::EntryOffsetOrSizeDoesntFit);
        }

        Ok(Self {
            raw_archive,
            mmap,
            entries_offset,
        })
    }

    /// get bytes from the given offset.
    /// ### SAFETY:
    /// because we validate archive before this call, it should be safe to call with any **valid** entry offset and size.
    pub(crate) fn get_bytes(&self, offset: usize, size: usize) -> &[u8] {
        debug_assert!(offset + size <= self.mmap.len());
        log::debug!("getting bytes from offset {offset} with size {size}");
        &self.mmap[offset..offset + size]
    }

    /// a simple function to get a slice from buffer with size 0
    pub(crate) fn get_empty_bytes(&self) -> &[u8] {
        log::debug!("getting a zero sized slice");
        &self.mmap[0..0]
    }

    /// retuturn a reference the underlying raw archive
    #[cfg(feature = "raw_structure")]
    pub fn raw_archive(&self) -> &RawArchive {
        &self.raw_archive
    }
}

#[inline]
fn validate_entries(raw_archive: &RawArchive, mmap: &[u8]) -> bool {
    match raw_archive {
        RawArchive::Obscure1(archive) => {
            fn check_entry(e: &obscure1::Entry, len: usize) -> bool {
                match &e.kind {
                    obscure1::EntryKind::Dir(e) => e.entries.iter().all(|e| check_entry(e, len)),
                    obscure1::EntryKind::File(e) => {
                        // somehow entries with uncompressed size zero have crazy compressed sizes
                        // so we just ignore them
                        e.uncompressed_size == 0 || (e.offset + e.compressed_size) as usize <= len
                    }
                }
            }

            archive.entries.iter().all(|e| check_entry(e, mmap.len()))
        }
        RawArchive::Obscure2(archive) => archive.entries.iter().all(|e| match &e.kind {
            obscure2::EntryKind::File(file) | obscure2::EntryKind::FileCompressed(file) => {
                (file.offset + file.compressed_size) as usize <= mmap.len()
            }
            _ => true,
        }),
    }
}
