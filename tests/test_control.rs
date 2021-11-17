use libmft2bodyfile::{Mft2BodyfileTask, PreprocessedMft};
use mft::MftParser;
use std::path::PathBuf;
use std::collections::hash_set::HashSet;
use bodyfile::Bodyfile3Line;
use std::convert::TryFrom;

#[macro_use]
extern crate more_asserts;

fn get_mft_file() -> PathBuf {
    let prj_root = env!("CARGO_MANIFEST_DIR");
    let mut mft_file = PathBuf::from(prj_root);
    mft_file.push("tests");
    mft_file.push("data");
    mft_file.push("control");
    mft_file.push("MFT");
    mft_file
}

fn get_parsed_mft() -> PreprocessedMft {
    let mft_file = get_mft_file();
    let parser = MftParser::from_path(&mft_file).unwrap();
    Mft2BodyfileTask::fill_preprocessed_mft(parser, None)
}

#[test]
fn test_root_entry() {
    let root_entries: Vec<Bodyfile3Line> = get_parsed_mft()
                        .iter_entries(false)
                        .map(|l| Bodyfile3Line::try_from(l.as_ref()).expect(l.as_ref()))
                        .filter(|l| l.get_inode() == "5")
                        .collect();
    
    //for f in get_parsed_mft().iter_entries(false) {
    //    eprintln!("ENTRY: {}", f);
    //}

    assert_ge!(root_entries.len(), 1);
    assert_le!(root_entries.len(), 4);
    for e in root_entries.iter() {
        assert_ge!(e.get_name().len(), 1);
        if e.get_name().len() == 1 {
            assert_eq!(e.get_name(), "/");
        } else {
            assert_eq!(e.get_name().starts_with("/ "), true);
        }
    }
}


#[test]
fn test_deleted_entries() {
    let expected_entries = vec![
        "/MVC-577V.MPG ($FILE_NAME) (deleted)",
        "/MVC-577V.MPG (deleted)",
        "/deleted.JPG ($FILE_NAME) (deleted)",
        "/deleted.JPG (deleted)",
        "/RECYCLER ($FILE_NAME) (deleted)",
        "/RECYCLER (deleted)",
        "/RECYCLER/S-1-5-21-3958095517-222395546-2225589205-500 ($FILE_NAME) (deleted)",
        "/RECYCLER/S-1-5-21-3958095517-222395546-2225589205-500 (deleted)",
        "/RECYCLER/S-1-5-21-3958095517-222395546-2225589205-500/desktop.ini ($FILE_NAME) (deleted)",
        "/RECYCLER/S-1-5-21-3958095517-222395546-2225589205-500/desktop.ini (deleted)",
        "/RECYCLER/S-1-5-21-3958095517-222395546-2225589205-500/INFO2 ($FILE_NAME) (deleted)",
        "/RECYCLER/S-1-5-21-3958095517-222395546-2225589205-500/INFO2 (deleted)"
    ];

    let mut deleted_entries: HashSet<String> = get_parsed_mft()
                        .iter_entries(false)
                        .map(|l| Bodyfile3Line::try_from(l.as_ref()).expect(l.as_ref()))
                        .filter(|l| l.get_name().contains("deleted"))
                        .map(|l| l.get_name().to_owned())
                        .collect();

    eprintln!("{:?}", deleted_entries);
    for entry in expected_entries {
        assert!(deleted_entries.contains(entry), "entry '{}' was not found", entry);
        deleted_entries.remove(entry);
    }
    assert!(deleted_entries.is_empty(), "the following entries were unexpected: '{:?}'", deleted_entries);
}