// some platforms also have padding which we dont handle here

import std.core;
import std.mem;

// Little Endian: 00 00 04 00
// Big Endian   : 00 04 00 00
u8 magic_byte @ 0x01 [[hidden]];
bool is_be = (magic_byte == 0x04);

if (is_be) {
    std::core::set_endian(std::mem::Endian::Big);
} else {
    std::core::set_endian(std::mem::Endian::Little);
}

struct Header {
    u8  magic[4];
    u32 zero;
    u32 entries_count;
    u32 entries_crc32;
};

struct FileEntry {
    s32 checksum;
    u32 uncompressed_size;
    u32 offset;
    u32 compressed_size;
    u8  bytes[compressed_size] @ offset;
};

struct DirEntry {
    u32 zero[2];
    u32 count;
    u32 index;
};

struct Entry {
    u32 name_crc32;
    u16 kind;
    u16 zero;
    match (kind) {
        (0): FileEntry file_entry;  // uncompressed file
        (1): FileEntry file_entry;  // compressed file
        (4): DirEntry dir_entry;    // directory
    }
};

struct Archive {
    Header  header;
    Entry   entries[header.entries_count];
};

Archive archive @ 0;
