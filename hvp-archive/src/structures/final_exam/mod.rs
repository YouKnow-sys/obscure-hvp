//! final exam hvp archive structure
//!
//! mostly similar to obscure 2, but with one new field in entry and include file names

use std::{
    io::{Read, Seek, SeekFrom},
    ops::Range,
};

use binrw::{BinResult, Endian, binrw};

use super::common;

const LITTLE_ENDIAN_MAGIC: [u8; 4] = [0, 0, 5, 0];
const BIG_ENDIAN_MAGIC: [u8; 4] = [0, 5, 0, 0];

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
#[br(stream = r, is_big = is_magic_big_endian(r)?)]
#[bw(is_big = self.endian() == Endian::Big)]
pub struct HvpArchive {
    #[bw(args(entries))]
    pub header: Header,
    pub names: Names,
    #[br(args(header.entries_count as _, Some(header.entries_crc32)))]
    #[br(parse_with = common::read_entries_with_validation)]
    #[br(assert(have_root_entry(&entries), "invalid final exam hvp, archive should start with a root directory entry"))]
    #[br(assert(names.validate_name_offsets(&entries), "invalid name offsets in the archive"))]
    pub entries: Vec<Entry>,
}

impl HvpArchive {
    pub(crate) fn endian(&self) -> Endian {
        get_endian_by_magic(self.header.magic)
    }
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
#[bw(import(entries: &[Entry]))]
pub struct Header {
    #[br(assert(magic == LITTLE_ENDIAN_MAGIC || magic == BIG_ENDIAN_MAGIC, "invalid magic value"))]
    magic: [u8; 4],
    #[br(assert(zero == 0))]
    zero: u32,
    #[br(assert(entries_count > 0, "invalid or empty hvp archive"))]
    pub entries_count: u32,
    #[br(assert(entries_crc32 > 0, "invalid archive, not a hvp file"))]
    #[bw(try_map = |_| common::generate_crc32(&entries, get_endian_by_magic(self.magic)))]
    pub entries_crc32: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct Names {
    #[br(temp)]
    #[bw(calc = bytes.len() as u32)]
    bytes_len: u32,
    #[br(count = bytes_len)]
    bytes: Vec<u8>,
}

impl Names {
    fn validate_name_offsets(&self, entries: &[Entry]) -> bool {
        for entry in entries {
            let offset = match &entry.kind {
                EntryKind::File(entry) => entry.name_offset,
                EntryKind::FileCompressed(entry) => entry.name_offset,
                EntryKind::Directory(entry) => entry.name_offset,
            };

            if offset > self.bytes.len() as u32 {
                return false;
            }

            let Some(name) = &self.bytes[offset as usize..].split(|&b| b == 0).next() else {
                return false;
            };

            if std::str::from_utf8(name).is_err() {
                return false;
            }
        }
        true
    }

    /// Get the name of an entry by its offset.
    /// ### SAFETY:
    /// because we validate names when parsing the archive, it should be safe to call with any **valid** entry name offset.
    pub fn get_name_by_offset(&self, offset: u32) -> &str {
        debug_assert!(offset <= self.bytes.len() as u32);

        let name = &self.bytes[offset as usize..]
            .split(|&b| b == 0)
            .next()
            .unwrap();

        let name = std::str::from_utf8(name)
            .ok()
            .expect("got invalid name in names section");

        name
    }
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
    pub name_offset: u32,
    pub offset: u32,
    pub compressed_size: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct DirEntry {
    #[br(assert(zero1 == 0))]
    zero1: u32,
    #[br(assert(zero2 == 0))]
    zero2: u32,
    pub name_offset: u32,
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

// PC(tested) use LittleEndian
// Maybe some other platform use BigEndian so I'm keeping it
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

#[inline(always)]
fn get_endian_by_magic(magic: [u8; 4]) -> Endian {
    match magic {
        LITTLE_ENDIAN_MAGIC => Endian::Little,
        BIG_ENDIAN_MAGIC => Endian::Big,
        _ => unreachable!(),
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
