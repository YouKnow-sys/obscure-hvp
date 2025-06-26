use std::io;

#[derive(Debug, thiserror::Error)]
pub enum RebuildError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    ArchiveLoadFailed(#[from] binrw::Error),
    #[error("zlib compression failed")]
    ZlibCompressionFailed(#[from] flate2::CompressError),
    #[error("lzo compression failed")]
    LzoCompressionFailed(#[from] lzokay_native::Error),
}
