//! common methods and types between games

use std::io::{Seek, SeekFrom, Write};

use binrw::{BinRead, BinResult, BinWrite, Endian, Error, VecArgs, parser, writer};

#[parser(reader, endian)]
pub fn read_string() -> BinResult<String> {
    let count = u32::read_options(reader, endian, ())? as usize;
    let pos = reader.stream_position()?;
    let bytes = Vec::<u8>::read_options(reader, endian, VecArgs { count, inner: () })?;
    String::from_utf8(bytes).map_err(|e| Error::Custom {
        pos,
        err: Box::new(e),
    })
}

#[writer(writer, endian)]
pub fn write_string(str: &String) -> BinResult<()> {
    (str.len() as u32).write_options(writer, endian, ())?;
    str.as_bytes().write_options(writer, endian, ())?;
    Ok(())
}

pub fn generate_crc32<D>(data: &D, endian: Endian) -> BinResult<u32>
where
    for<'a> D: BinWrite<Args<'a> = ()>,
{
    let mut writer = DummyCrc32Writer::new();
    data.write_options(&mut writer, endian, ())?;
    Ok(writer.checksum())
}

/// A dummy writer that we use only to caculate crc32 checksum
pub struct DummyCrc32Writer {
    hasher: crc32fast::Hasher,
    pos: u64,
}

impl DummyCrc32Writer {
    pub fn new() -> Self {
        Self {
            hasher: crc32fast::Hasher::new(),
            pos: 0,
        }
    }

    pub fn checksum(self) -> u32 {
        self.hasher.finalize()
    }
}

impl Write for DummyCrc32Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.update(buf);
        self.pos += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for DummyCrc32Writer {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let SeekFrom::Current(0) = pos else {
            unimplemented!("this writer doesn't support seek")
        };

        Ok(self.pos)
    }
}
