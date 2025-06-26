//! obscure 2 hvp archive structure

use std::ops::Range;

use binrw::{BinResult, BinWrite, Endian, binrw};

use super::common;

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct HvpArchive {
    pub header: Header,
    #[br(count  = header.entries_count)]
    #[br(assert(have_root_entry(&entries), "invalid obscure 2 hvp, archive should start with a root directory entry"))]
    pub entries: Vec<Entry>,
}

impl HvpArchive {
    pub fn update_checksums(&mut self, endian: Endian) -> BinResult<()> {
        let mut writer = common::DummyCrc32Writer::new();
        self.entries.write_options(&mut writer, endian, ())?;
        self.header.entries_crc32 = writer.checksum();
        Ok(())
    }
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
#[brw(magic = b"\x00\x00\x04\x00")]
pub struct Header {
    #[br(assert(zero == 0))]
    zero: u32,
    #[br(assert(entries_count > 0, "invalid or empty hvp archive"))]
    pub entries_count: u32,
    #[br(assert(entries_crc32 > 0, "invalid archive, not a hvp file"))]
    pub entries_crc32: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct Entry {
    pub name_crc32: u32,
    pub kind: EntryKind,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub enum EntryKind {
    #[brw(magic = 0u32)]
    File(FileEntry),
    #[brw(magic = 1u32)]
    FileCompressed(FileEntry),
    #[brw(magic = 4u32)]
    Directory(DirEntry),
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct FileEntry {
    pub checksum: i32,
    pub uncompressed_size: u32,
    pub offset: u32,
    pub compressed_size: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct DirEntry {
    #[br(assert(zero1 == 0))]
    pub(crate) zero1: u32,
    #[br(assert(zero2 == 0))]
    pub(crate) zero2: u32,
    #[br(assert(count > 0, "invalid archive, directory can't have zero entries"))]
    pub count: u32,
    pub index: u32,
}

impl DirEntry {
    pub fn entries_range(&self) -> Range<usize> {
        let start = self.index as usize;
        let end = start + self.count as usize;
        start..end
    }
}

fn have_root_entry(entries: &[Entry]) -> bool {
    matches!(
        entries[0],
        Entry {
            name_crc32: 0,
            kind: EntryKind::Directory(DirEntry { index: 1, .. }),
        }
    )
}
