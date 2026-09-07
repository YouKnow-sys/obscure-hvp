// some platforms also have padding whcich we dont handle here

import std.core;
import std.mem;

// Little Endian: 00 00 05 00
// Big Endian   : 00 05 00 00
u8 magic_byte @ 0x01 [[hidden]];
bool is_be = magic_byte == 0x05;

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

struct Names {
    u32 len;
    u8  bytes[len];
};

fn format_name(auto n) {
    return n.name;
};

struct NameRef {
    u32 offset;
    char name[] @ (addressof(parent.parent.parent.names) + 4 + offset);
} [[inline, format("format_name")]];

struct FileEntry {
    s32 checksum;
    u32 uncompressed_size;
    NameRef name;
    u32 offset;
    u32 compressed_size;
    u8 bytes[compressed_size] @ offset;
};

struct DirEntry {
    u32 zero[2];
    NameRef name;
    u32 count;
    u32 index;
};

struct Entry {
    u32 name_crc32;
    u32 kind;
    match (kind) {
        (0): FileEntry file_entry;  // uncompressed file
        (1): FileEntry file_entry;  // compressed file
        (4): DirEntry dir_entry;    // directory
    }
};

struct Archive {
    Header  header;
    Names   names;
    Entry   entries[header.entries_count];
};

Archive archive @ 0;
