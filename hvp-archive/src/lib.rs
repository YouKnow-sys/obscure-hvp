pub use utils::try_detect_game;

pub mod archive;
pub mod provider;

#[cfg(feature = "raw_structure")]
pub mod structures;
#[cfg(not(feature = "raw_structure"))]
mod structures;

mod utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game {
    Obscure1,
    Obscure2,
}
