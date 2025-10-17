use std::{
    fs::File,
    io::{Cursor, Write},
};

use hvp_archive::{
    Game,
    archive::{Archive, Metadata, rebuild_progress::RebuildProgress},
    provider::ArchiveProvider,
};

mod constants;

fn load() -> ArchiveProvider {
    let file = File::open(constants::FINAL_EXAM_HVP).expect("failed to open file");
    ArchiveProvider::new(file, Some(Game::FinalExam))
        .expect("failed to load hvp archive using provider")
}

#[test]
fn load_and_check_final_exam() {
    let provider = load();
    let archive = Archive::new(&provider);

    // check archive metadata

    assert_eq!(
        archive.metadata(),
        Metadata {
            dir_count: 4,
            file_count: 13,
            game: Game::FinalExam
        },
        "archive metadata doesn't match with the expected metadata"
    );

    // check whatever checksums are valid

    assert!(
        archive.entries_checksum_match(),
        "entries checksum doesn't match"
    );
}

#[test]
fn rebuild_final_exam() {
    let provider = load();
    let archive = Archive::new(&provider);

    let org_archive = std::fs::read(constants::FINAL_EXAM_HVP).expect("failed to open file");
    let mut writer = Cursor::new(Vec::with_capacity(org_archive.len()));
    archive
        .rebuild(&mut writer, EmptyProgress)
        .expect("failed to rebuild archive");

    writer.flush().unwrap();
    let rebuild_archive = writer.into_inner();

    assert_eq!(
        org_archive, rebuild_archive,
        "the original archive doesn't match the new generated archive"
    );
}

struct EmptyProgress;

impl RebuildProgress for EmptyProgress {
    fn inc(&self, _: Option<String>) {}
    fn inc_n(&self, _: usize, _: Option<String>) {}
}
