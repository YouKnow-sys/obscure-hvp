use binrw::binrw;

use crate::utils;

#[binrw]
#[brw(magic = b"HV PackFile\0")]
#[derive(Debug)]
pub struct HvpArchive {
    pub version: u16,
    have_unknown_section: u16,
    #[br(assert(root_count > 0, "invalid archive, not a hvp file"))]
    pub root_count: u32,
    #[br(assert(all_count > 0, "invalid archive, not a hvp file"))]
    pub all_count: u32,
    #[br(assert(file_count > 0, "invalid archive, not a hvp file"))]
    pub file_count: u32,
    #[br(assert(file_count > 0, "invalid archive, not a hvp file"))]
    pub data_offset: u32,
    #[br(if(have_unknown_section == 1))]
    pub extra_info: Option<[u8; 8]>,
    #[br(count = root_count)]
    pub entries: Vec<Entry>,
}

#[binrw]
#[derive(Debug)]
pub struct Entry {
    #[br(assert(entry_size > 0, "invalid entry in archive"))]
    entry_size: u32,
    pub entry_type: u8,
    #[br(args_raw(entry_type))]
    pub kind: EntryKind,
}

#[binrw]
#[derive(Debug)]
#[br(import_raw(entry_type: u8))]
pub enum EntryKind {
    #[br(pre_assert(entry_type == 0))]
    Dir(DirEntry),
    File(FileEntry),
}

#[binrw]
#[derive(Debug)]
pub struct FileEntry {
    #[br(map = |v: u32| v > 0)]
    #[bw(map = |v| *v as u32)]
    pub is_compressed: bool,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub hash: u32, // ?
    pub offset: u32,
    #[br(parse_with(utils::read_string))]
    #[bw(write_with(utils::write_string))]
    pub name: String,
}

#[binrw]
#[derive(Debug)]
pub struct DirEntry {
    #[br(assert(zero == 0))]
    zero: u32,
    #[br(temp)]
    #[bw(calc = entries.len() as u32)]
    pub count: u32,
    #[br(parse_with(utils::read_string))]
    #[bw(write_with(utils::write_string))]
    pub name: String,
    #[br(count = count)]
    pub entries: Vec<Entry>,
}
