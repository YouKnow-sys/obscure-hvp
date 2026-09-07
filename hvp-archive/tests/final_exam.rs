use std::{
    fs::File,
    io::{Cursor, Write},
};

use hvp_archive::{
    Game,
    archive::{Archive, Metadata, entry::UpdateKind},
    provider::ArchiveProvider,
};

mod common;

fn load() -> ArchiveProvider {
    let file = File::open(common::FINAL_EXAM_HVP).expect("failed to open file");
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

    // rebuild the archive as is without any changes
    let org_archive = std::fs::read(common::FINAL_EXAM_HVP).expect("failed to open file");
    let mut writer = Cursor::new(Vec::with_capacity(org_archive.len()));
    archive
        .rebuild(&mut writer, common::EmptyProgress)
        .expect("failed to rebuild archive");

    writer.flush().unwrap();
    let rebuild_archive = writer.into_inner();

    assert_eq!(
        org_archive, rebuild_archive,
        "the original archive doesn't match the new generated archive"
    );
}

#[test]
fn rebuild_final_exam_with_updated_entries() {
    let provider = load();
    let archive = Archive::new(&provider);
    let mut archive_to_update = Archive::new(&provider);

    // update the entries (same unchanged original entries)
    archive_to_update
        .files_mut()
        .zip(archive.files())
        .for_each(|(mut fu, f)| fu.update(UpdateKind::Bytes(f.get_bytes().unwrap().to_vec())));

    let org_archive = std::fs::read(common::FINAL_EXAM_HVP).expect("failed to open file");
    let mut writer = Cursor::new(Vec::with_capacity(org_archive.len()));
    archive_to_update
        .rebuild(&mut writer, common::EmptyProgress)
        .expect("failed to rebuild archive");

    writer.flush().unwrap();
    let rebuild_archive = writer.into_inner();

    assert_eq!(
        org_archive, rebuild_archive,
        "the original archive doesn't match the new generated archive"
    );
}
