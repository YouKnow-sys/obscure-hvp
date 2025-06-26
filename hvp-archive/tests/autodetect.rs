use std::{fs::File, io::Cursor};

use hvp_archive::{Game, try_detect_game};

mod constants;

#[test]
fn autodetect_obscure1() {
    let obscure1 = {
        let mut file = File::open(constants::OBSCURE1_HVP).expect("failed to open file");
        try_detect_game(&mut file).expect("failed to parse obscure1 archive")
    };

    assert_eq!(
        obscure1,
        Some(Game::Obscure1),
        "failed to detect obscure1 hvp archive"
    );
}

#[test]
fn autodetect_obscure2() {
    let obscure2 = {
        let mut file = File::open(constants::OBSCURE2_HVP).expect("failed to open file");
        try_detect_game(&mut file).expect("failed to parse obscure2 archive")
    };

    assert_eq!(
        obscure2,
        Some(Game::Obscure2),
        "failed to detect obscure2 hvp archive"
    );
}

#[test]
fn autodetect_invalid() {
    let invalid = {
        let mut reader = Cursor::new([0, 0, 0, 0, 0, 0, 0, 0]);
        try_detect_game(&mut reader).expect("failed to parse invalid data")
    };

    assert_eq!(invalid, None, "the input should be detected as invalid");
}
