use std::{
    borrow::Cow,
    fmt::Debug,
    fs, io,
    path::{Path, PathBuf},
};

use binrw::Endian;

use crate::structures;

/// you can just put the bytes that you want the archive to update from here
/// or a path to a file
#[derive(Clone)]
pub enum UpdateKind {
    Bytes(Vec<u8>),
    File(PathBuf),
}

impl UpdateKind {
    /// return the content of update as a vector of bytes
    pub fn to_bytes(&self) -> io::Result<Cow<'_, [u8]>> {
        match self {
            UpdateKind::Bytes(bytes) => Ok(Cow::Borrowed(bytes)),
            UpdateKind::File(path) => fs::read(path).map(Cow::Owned),
        }
    }
}

impl Debug for UpdateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytes(_) => f.debug_tuple("Bytes").field(&"...").finish(),
            Self::File(path) => f.debug_tuple("File").field(path).finish(),
        }
    }
}

/// compression type
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    /// used by obscure 1
    Zlib,
    /// used by obscure 2
    Lzo,
}

/// info about the compression
#[derive(Debug, Clone, Copy)]
pub struct CompressionInfo {
    pub uncompressed_size: u32,
    pub compression_type: CompressionType,
}

/// file entry, contain info about the file and its raw bytes.
/// can also be used to decompress the bytes if the entry is
/// compressed.
#[derive(Clone)]
pub struct FileEntry<'p> {
    pub(crate) name: String,
    pub(crate) compression_info: Option<CompressionInfo>,
    pub(crate) checksum: i32,
    pub(crate) endian: Endian,
    pub raw_bytes: &'p [u8],
    /// if this path is set we replace the entry data with file from this path
    pub update: Option<UpdateKind>,
}

impl FileEntry<'_> {
    /// name of the entry
    pub fn name(&self) -> &str {
        &self.name
    }

    /// whatever the entry is compressed or not
    pub fn is_compressed(&self) -> bool {
        self.compression_info.is_some()
    }

    /// get the bytes of the entry. decompress if needed
    pub fn get_bytes(&self) -> Result<Cow<'_, [u8]>, DecompressError> {
        match self.compression_info {
            Some(info) => decompress_buf(self.raw_bytes, info).map(Cow::Owned),
            None => Ok(Cow::Borrowed(self.raw_bytes)),
        }
    }

    /// check whatever the checksum match
    pub fn checksum_match(&self) -> bool {
        structures::checksum::bytes_sum(self.raw_bytes, self.endian) == self.checksum
    }
}

impl Debug for FileEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileEntry")
            .field("name", &self.name)
            .field("compression_info", &self.compression_info)
            .field("checksum", &self.checksum)
            .field("raw_bytes", &format!("[u8; {}]", self.raw_bytes.len()))
            .field("update", &self.update)
            .finish()
    }
}

/// directory entry, contain the name of directory and entries inside it
#[derive(Clone)]
pub struct DirEntry<'p> {
    pub name: String,
    pub entries: Vec<Entry<'p>>,
}

impl Debug for DirEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirEntry")
            .field("name", &self.name)
            .field("entries", &format!("[Entry; {}]", self.entries.len()))
            .finish()
    }
}

/// full file entry contain full path to a file and other infos about it
pub struct FullFileEntry<'p> {
    pub path: PathBuf,
    pub(super) compression_info: Option<CompressionInfo>,
    pub(super) checksum: i32,
    pub(super) endian: Endian,
    pub raw_bytes: &'p [u8],
}

impl FullFileEntry<'_> {
    /// get the bytes of the entry. decompress if needed
    pub fn get_bytes(&self) -> Result<Cow<'_, [u8]>, DecompressError> {
        match self.compression_info {
            Some(info) => decompress_buf(self.raw_bytes, info).map(Cow::Owned),
            None => Ok(Cow::Borrowed(self.raw_bytes)),
        }
    }

    /// whatever the entry is compressed or not
    pub fn is_compressed(&self) -> bool {
        self.compression_info.is_some()
    }

    /// check whatever the checksum match
    pub fn checksum_match(&self) -> bool {
        structures::checksum::bytes_sum(self.raw_bytes, self.endian) == self.checksum
    }
}

impl Debug for FullFileEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullFileEntry")
            .field("path", &self.path.display())
            .field("compression_info", &self.compression_info)
            .field("checksum", &self.checksum)
            .field("raw_bytes", &format!("[u8; {}]", self.raw_bytes.len()))
            .finish()
    }
}

/// full file entry contain full path to a file and a mutable reference to the file itself
/// that can be used to update it
pub struct FullFileEntryMut<'a, 'p> {
    pub path: PathBuf,
    pub(crate) entry: &'a mut FileEntry<'p>,
}

impl FullFileEntryMut<'_, '_> {
    /// get the bytes of the entry. decompress if needed
    pub fn get_bytes(&self) -> Result<Cow<'_, [u8]>, DecompressError> {
        self.entry.get_bytes()
    }

    /// get raw bytes of the entry
    pub fn raw_bytes(&self) -> &[u8] {
        self.entry.raw_bytes
    }

    /// whatever the entry is compressed or not
    pub fn is_compressed(&self) -> bool {
        self.entry.compression_info.is_some()
    }

    /// check whatever the checksum match
    pub fn checksum_match(&self) -> bool {
        structures::checksum::bytes_sum(self.entry.raw_bytes, self.entry.endian)
            == self.entry.checksum
    }

    /// update the entry
    pub fn update(&mut self, update: impl Into<Option<UpdateKind>>) {
        self.entry.update = update.into();
    }
}

impl Debug for FullFileEntryMut<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullFileEntryMut")
            .field("path", &self.path.display())
            .field("entry", &self.entry)
            .finish()
    }
}

/// A entry can be either a file or a directory of entries
#[derive(Debug, Clone)]
pub enum Entry<'p> {
    File(FileEntry<'p>),
    Dir(DirEntry<'p>),
}

impl<'p> Entry<'p> {
    /// flatten the entry to its files
    pub fn flatten_to_files(&self) -> Vec<FullFileEntry<'p>> {
        fn file<'p>(entry: &FileEntry<'p>, parent: Option<&Path>) -> FullFileEntry<'p> {
            let path = match parent {
                Some(path) => path.join(&entry.name),
                None => PathBuf::from(&entry.name),
            };

            FullFileEntry {
                path,
                compression_info: entry.compression_info,
                checksum: entry.checksum,
                endian: entry.endian,
                raw_bytes: entry.raw_bytes,
            }
        }

        fn dir<'p>(entry: &DirEntry<'p>, parent: Option<&Path>) -> Vec<FullFileEntry<'p>> {
            let path = match parent {
                Some(path) => path.join(&entry.name),
                None => PathBuf::from(&entry.name),
            };

            let mut result = Vec::new();

            for entry in &entry.entries {
                match entry {
                    Entry::File(entry) => result.push(file(entry, Some(&path))),
                    Entry::Dir(entry) => result.extend(dir(entry, Some(&path))),
                }
            }

            result
        }

        match self {
            Entry::File(entry) => vec![file(entry, None)],
            Entry::Dir(entry) => dir(entry, None),
        }
    }

    /// flatten the entry to its files with mutable access
    pub fn flatten_to_files_mut(&mut self) -> Vec<FullFileEntryMut<'_, 'p>> {
        fn file<'a, 'p>(
            entry: &'a mut FileEntry<'p>,
            parent: Option<&Path>,
        ) -> FullFileEntryMut<'a, 'p> {
            let path = match parent {
                Some(path) => path.join(&entry.name),
                None => PathBuf::from(&entry.name),
            };

            FullFileEntryMut { path, entry }
        }

        fn dir<'a, 'p>(
            entry: &'a mut DirEntry<'p>,
            parent: Option<&Path>,
        ) -> Vec<FullFileEntryMut<'a, 'p>> {
            let path = match parent {
                Some(path) => path.join(&entry.name),
                None => PathBuf::from(&entry.name),
            };

            let mut result = Vec::new();

            for entry in entry.entries.iter_mut() {
                match entry {
                    Entry::File(entry) => result.push(file(entry, Some(&path))),
                    Entry::Dir(entry) => result.extend(dir(entry, Some(&path))),
                }
            }

            result
        }

        match self {
            Entry::File(entry) => vec![file(entry, None)],
            Entry::Dir(entry) => dir(entry, None),
        }
    }
}

/// errors that can happen during decompression
#[derive(Debug, thiserror::Error)]
pub enum DecompressError {
    #[error("failed to decompress using zlib")]
    Zlib(#[from] flate2::DecompressError),
    #[error("failed to decompress using lzo")]
    Lzo(#[from] lzokay_native::Error),
}

#[inline(always)]
fn decompress_buf(
    input: &[u8],
    compression_info: CompressionInfo,
) -> Result<Vec<u8>, DecompressError> {
    let uncompressed_size = compression_info.uncompressed_size as _;
    let output = match compression_info.compression_type {
        CompressionType::Zlib => {
            let mut output = vec![0_u8; uncompressed_size];
            flate2::Decompress::new(true).decompress(
                input,
                &mut output,
                flate2::FlushDecompress::Finish,
            )?;
            output
        }
        CompressionType::Lzo => lzokay_native::decompress_all(input, Some(uncompressed_size))?,
    };
    Ok(output)
}
