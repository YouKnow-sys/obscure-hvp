use std::io;

/// errors that can happen during rebuilding of a archive
#[derive(Debug, thiserror::Error)]
pub enum RebuildError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    BinRW(#[from] binrw::Error),
    #[error("zlib compression failed")]
    ZlibCompressionFailed(#[from] flate2::CompressError),
}
