//! obscure 1 hvp archive structure

use binrw::{Endian, binrw};

use super::common;

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct HvpArchive {
    pub header: Header,
    #[br(if(header.minor_version == 1))]
    #[bw(args(header, entries))]
    pub checksums: Option<Crc32>,
    #[br(args(header.root_count as _, checksums.as_ref().map(|c| c.entries)))]
    #[br(parse_with = common::read_entries_with_validation)]
    pub entries: Vec<Entry>,
}

#[binrw]
#[brw(magic = b"HV PackFile\0")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct Header {
    pub major_version: u16,
    pub minor_version: u16,
    #[br(assert(root_count > 0, "invalid archive, not a hvp file"))]
    pub root_count: u32,
    #[br(assert(all_count > 0, "invalid archive, not a hvp file"))]
    pub all_count: u32,
    #[br(assert(file_count > 0, "invalid archive, not a hvp file"))]
    pub file_count: u32,
    #[br(assert(file_count > 0, "invalid archive, not a hvp file"))]
    pub data_offset: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
#[bw(import(in_header: &Header, in_entries: &[Entry]))]
pub struct Crc32 {
    #[bw(try_map = |_| common::generate_crc32(&in_header, Endian::Big))]
    pub header: u32,
    #[bw(try_map = |_| common::generate_crc32(&in_entries, Endian::Big))]
    pub entries: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct Entry {
    #[br(assert(entry_size > 0, "invalid entry in archive"))]
    entry_size: u32,
    pub kind: EntryKind,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub enum EntryKind {
    #[brw(magic = 0u8)]
    Dir(DirEntry),
    #[brw(magic = 1u8)]
    File(FileEntry),
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct FileEntry {
    #[br(map = |v: u32| v > 0)]
    #[bw(map = |v| *v as u32)]
    pub is_compressed: bool,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub checksum: i32,
    pub offset: u32,
    #[br(parse_with(common::read_string))]
    #[bw(write_with(common::write_string))]
    pub name: String,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "raw_structure", derive(serde::Serialize))]
pub struct DirEntry {
    #[br(assert(zero == 0))]
    zero: u32,
    #[br(temp)]
    #[bw(calc = entries.len() as u32)]
    pub count: u32,
    #[br(parse_with(common::read_string))]
    #[bw(write_with(common::write_string))]
    pub name: String,
    #[br(count = count)]
    pub entries: Vec<Entry>,
}
