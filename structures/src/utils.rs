use binrw::{BinRead, BinResult, BinWrite, Error, VecArgs, parser, writer};

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
