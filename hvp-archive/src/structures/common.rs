//! common methods and types between games

use std::io::{Read, Seek, SeekFrom, Write};

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

/// a reader that let us read entries and also validate their crc32
#[binrw::parser(reader, endian)]
pub fn read_entries_with_validation<T>(
    count: usize,
    expected_crc32: Option<u32>,
) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()> + 'static,
{
    let mut reader = Crc32Reader::new(reader)?;
    let pos = reader.stream_position()?;

    let entries = <Vec<T>>::read_options(
        &mut reader,
        endian,
        VecArgs::builder().count(count).finalize(),
    )?;

    let Some(expected_crc32) = expected_crc32 else {
        return Ok(entries);
    };

    let entries_crc32 = reader.hash();
    if entries_crc32 != expected_crc32 {
        return Err(Error::AssertFail {
            pos,
            message: format!(
                "field have invalid crc32, expected {expected_crc32} but got {entries_crc32}"
            ),
        });
    }

    Ok(entries)
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

/// A reader that also generate crc32 of the content it read
pub struct Crc32Reader<'a, R: Read + Seek> {
    reader: &'a mut R,
    hasher: crc32fast::Hasher,
    pos: u64,
    checksum_pos: u64,
}

impl<'a, R: Read + Seek> Crc32Reader<'a, R> {
    pub fn new(reader: &'a mut R) -> std::io::Result<Self> {
        let pos = reader.stream_position()?;
        Ok(Self {
            reader,
            hasher: crc32fast::Hasher::new(),
            pos,
            checksum_pos: pos,
        })
    }

    pub fn hash(self) -> u32 {
        self.hasher.finalize()
    }
}

impl<R: Read + Seek> Read for Crc32Reader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.reader.read(buf)?;
        self.pos += read as u64;

        // HACK: we do this because `magic` in binrw seek back
        // and we don't want to update our hasher with the same
        // value twice
        if self.pos != self.checksum_pos {
            self.hasher.update(&buf[..read]);
            self.checksum_pos += read as u64;
        }

        Ok(read)
    }
}

impl<R: Read + Seek> Seek for Crc32Reader<'_, R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let pos = self.reader.seek(pos)?;
        self.pos = pos;
        Ok(pos)
    }
}
