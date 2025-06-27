//! obscure 2 hvp archive structure

use std::ops::Range;

use binrw::{BinResult, BinWrite, Endian, binrw};

use super::common;

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
#[brw(little)] // doesn't really matter
pub enum HvpArchive {
    #[brw(magic = b"\x00\x00\x04\x00")]
    #[brw(little)]
    LittleEndian(HvpArchiveInner),
    #[brw(magic = b"\x00\x04\x00\x00")]
    #[brw(big)]
    BigEndian(HvpArchiveInner),
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct HvpArchiveInner {
    pub header: Header,
    #[br(count  = header.entries_count)]
    #[br(assert(have_root_entry(&entries), "invalid obscure 2 hvp, archive should start with a root directory entry"))]
    pub entries: Vec<Entry>,
}

impl HvpArchive {
    #[inline(always)]
    pub fn entries(&self) -> &[Entry] {
        match self {
            HvpArchive::LittleEndian(inner) => &inner.entries,
            HvpArchive::BigEndian(inner) => &inner.entries,
        }
    }

    #[inline(always)]
    pub fn entries_mut(&mut self) -> &mut [Entry] {
        match self {
            HvpArchive::LittleEndian(inner) => &mut inner.entries,
            HvpArchive::BigEndian(inner) => &mut inner.entries,
        }
    }

    #[cfg(feature = "raw_structure")]
    pub fn header(&self) -> &Header {
        match self {
            HvpArchive::LittleEndian(inner) => &inner.header,
            HvpArchive::BigEndian(inner) => &inner.header,
        }
    }

    pub fn header_mut(&mut self) -> &mut Header {
        match self {
            HvpArchive::LittleEndian(inner) => &mut inner.header,
            HvpArchive::BigEndian(inner) => &mut inner.header,
        }
    }

    pub fn update_checksums(&mut self) -> BinResult<()> {
        let mut writer = common::DummyCrc32Writer::new();

        let (entries, endian) = match self {
            HvpArchive::LittleEndian(inner) => (&inner.entries, Endian::Little),
            HvpArchive::BigEndian(inner) => (&inner.entries, Endian::Big),
        };

        entries.write_options(&mut writer, endian, ())?;
        self.header_mut().entries_crc32 = writer.checksum();
        Ok(())
    }
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
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
    #[brw(magic = 0u16)]
    File(FileEntry),
    #[brw(magic = 1u16)]
    FileCompressed(FileEntry),
    #[brw(magic = 4u16)]
    Directory(DirEntry),
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct FileEntry {
    #[br(assert(zero == 0))]
    zero: i16,
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
    zero1: i16,
    #[br(assert(zero2 == 0))]
    zero2: u32,
    #[br(assert(zero3 == 0))]
    zero3: u32,
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
