//! obscure 2 hvp archive structure

use std::{
    io::{Read, Seek, SeekFrom},
    ops::Range,
};

use binrw::{BinResult, BinWrite, Endian, binrw};

use super::common;

const LITTLE_ENDIAN_MAGIC: [u8; 4] = [0, 0, 4, 0];
const BIG_ENDIAN_MAGIC: [u8; 4] = [0, 4, 0, 0];

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
#[br(stream = r, is_big = is_magic_big_endian(r)?)]
#[bw(is_big = self.endian() == Endian::Big)]
pub struct HvpArchive {
    pub header: Header,
    // TODO: add entries checksum validation
    #[br(count  = header.entries_count)]
    #[br(assert(have_root_entry(&entries), "invalid obscure 2 hvp, archive should start with a root directory entry"))]
    pub entries: Vec<Entry>,
}

impl HvpArchive {
    pub(crate) fn endian(&self) -> Endian {
        match self.header.magic {
            LITTLE_ENDIAN_MAGIC => Endian::Little,
            BIG_ENDIAN_MAGIC => Endian::Big,
            _ => unreachable!(),
        }
    }

    pub fn update_checksums(&mut self) -> BinResult<()> {
        let mut writer = common::DummyCrc32Writer::new();

        self.entries.write_options(&mut writer, self.endian(), ())?;
        self.header.entries_crc32 = writer.checksum();

        Ok(())
    }
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct Header {
    #[br(assert(magic == LITTLE_ENDIAN_MAGIC || magic == BIG_ENDIAN_MAGIC, "invalid magic value"))]
    magic: [u8; 4],
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
    zero: u16,
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
    zero1: u16,
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

// PC, PS2 and PSP use LittleEndian
// Wii use BigEndian
fn is_magic_big_endian<R: Read + Seek>(reader: &mut R) -> BinResult<bool> {
    let pos = reader.stream_position()?;

    let mut buf = [0_u8; 4];
    reader.read_exact(&mut buf)?;
    reader.seek(SeekFrom::Start(pos))?;

    match buf {
        LITTLE_ENDIAN_MAGIC => Ok(false),
        BIG_ENDIAN_MAGIC => Ok(true),
        _ => Err(binrw::Error::BadMagic {
            pos,
            found: Box::new(buf),
        }),
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
