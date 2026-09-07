#pragma endian big

fn format_sized_string(auto s) {
    return s.value;
};

struct SizedString {
    u32 len;
    char value[len];
} [[format("format_sized_string")]];

struct Header {
    char magic[12]; // HV PackFile\0
    u16 major_version;
    u16 minor_version;
    u32 root_count;
    u32 all_count;
    u32 file_count;
    u32 data_offset;
};

struct Checksums {
    u32 header;
    u32 entries;
};

struct FileEntry {
    u32 is_compressed;
    u32 compressed_size;
    u32 uncompressed_size;
    s32 checksum;
    u32 offset;
    SizedString name;
    u8 bytes[compressed_size] @ offset;
};

using Entry;

struct DirEntry {
    u32 zero;
    u32 count;
    SizedString name;
    Entry entries[count];
    
};

struct Entry {
    u32 entry_size;
    u8  kind;
    match (kind) {
        (0): DirEntry dir_entry;
        (1): FileEntry file_entry;
    }
};

struct Archive {
    Header header;
    Checksums checksums;
    Entry entries[header.root_count];
};

Archive archive @ $;