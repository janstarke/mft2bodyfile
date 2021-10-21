use libmft2bodyfile::{Mft2BodyfileTask, PreprocessedMft};
use mft::MftParser;
use std::path::PathBuf;
use std::collections::hash_set::HashSet;

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

#[allow(dead_code)]
struct ParsedBodyfileLine {
    pub md5:String,
    pub name:String,
    pub inode:String,
    pub mode_as_string:String,
    pub uid:String,
    pub gid:String,
    pub size:String,
    pub atime:String,
    pub mtime:String,
    pub ctime:String,
    pub crtime:String,
}

impl ParsedBodyfileLine {
    pub fn from(line: &str) -> Self {
        let parts: Vec<&str> = line.split("|").collect();
        Self {
            md5:            parts[ 0].to_owned(),
            name:           parts[ 1].to_owned(),
            inode:          parts[ 2].to_owned(),
            mode_as_string: parts[ 3].to_owned(),
            uid:            parts[ 4].to_owned(),
            gid:            parts[ 5].to_owned(),
            size:           parts[ 6].to_owned(),
            atime:          parts[ 7].to_owned(),
            mtime:          parts[ 8].to_owned(),
            ctime:          parts[ 9].to_owned(),
            crtime:         parts[10].to_owned(),
        }
    }
}

fn get_parsed_mft() -> PreprocessedMft {
    let mft_file = get_mft_file();
    let parser = MftParser::from_path(&mft_file).unwrap();
    Mft2BodyfileTask::fill_preprocessed_mft(parser, None)
}

#[test]
fn test_root_entry() {
    let mft_file = get_mft_file();

    let root_entries: Vec<ParsedBodyfileLine> = get_parsed_mft()
                        .iter_entries(false)
                        .map(|l| ParsedBodyfileLine::from(&l))
                        .filter(|l| l.inode == "5")
                        .collect();
    
    assert_ge!(root_entries.len(), 1);
    assert_le!(root_entries.len(), 4);
    for e in root_entries.iter() {
        assert_ge!(e.name.len(), 1);
        if e.name.len() == 1 {
            assert_eq!(e.name, "/");
        } else {
            assert_eq!(e.name.starts_with("/ "), true);
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
                        .map(|l| ParsedBodyfileLine::from(&l))
                        .filter(|l| l.name.contains("deleted"))
                        .map(|l| l.name)
                        .collect();

    eprintln!("{:?}", deleted_entries);
    for entry in expected_entries {
        assert!(deleted_entries.contains(entry), "entry '{}' was not found", entry);
        deleted_entries.remove(entry);
    }
    assert!(deleted_entries.is_empty(), "the following entries were unexpected: '{:?}'", deleted_entries);
}