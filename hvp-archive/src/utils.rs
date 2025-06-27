use std::io::{Read, Seek, SeekFrom};

use crate::Game;

/// try to detect the game from the given reader.
/// this function will restore reader position after trying to detect the game.
pub fn try_detect_game<R: Read + Seek>(reader: &mut R) -> std::io::Result<Option<Game>> {
    let pos = reader.stream_position()?;
    reader.seek(SeekFrom::Start(0))?;
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    reader.seek(SeekFrom::Start(pos))?;
    match &buf {
        b"HV PackF" => Ok(Some(Game::Obscure1)),
        b"\x00\x00\x04\x00\x00\x00\x00\x00" | b"\x00\x04\x00\x00\x00\x00\x00\x00" => {
            Ok(Some(Game::Obscure2))
        }
        _ => Ok(None),
    }
}
