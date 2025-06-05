use binrw::binrw;

#[binrw]
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct HvpArchive {
    pub header: Header,
    #[br(count = header.entries_count)]
    pub entries: Vec<Entry>,
}

#[binrw]
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[brw(magic = b"\x00\x00\x04\x00")]
pub struct Header {
    #[br(assert(zero == 0))]
    zero: u32,
    #[br(assert(entries_count > 0, "invalid archive, not a hvp file"))]
    pub entries_count: u32,
    #[br(assert(entries_crc > 0, "invalid archive, not a hvp file"))]
    entries_crc: u32,
}

#[binrw]
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Entry {
    pub crc: u32,
    pub kind: EntryKind,
}

#[binrw]
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EntryKind {
    #[br(magic = 0u32)]
    File(FileEntry),
    #[br(magic = 1u32)]
    FileCompressed(FileEntry),
    #[br(magic = 4u32)]
    Directory(DirEntry),
}

#[binrw]
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FileEntry {
    pub checksum: u32, // ?
    pub uncompressed_size: u32,
    pub offset: u32,
    pub compressed_size: u32,
}

#[binrw]
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DirEntry {
    #[br(assert(zero1 == 0))]
    zero1: u32,
    #[br(assert(zero2 == 0))]
    zero2: u32,
    #[br(assert(entries_count > 0, "invalid archive, directory can't have zero entries"))]
    pub entries_count: u32,
    pub index: u32,
}
