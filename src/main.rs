use mft::MftParser;
use mft::attribute::MftAttributeContent;
use std::path::PathBuf;
use std::collections::HashMap;
use winstructs::ntfs::mft_reference::MftReference;
use argparse::{ArgumentParser, Store};
use anyhow::Result;

struct Timestamps {
    atime: i64,
    mtime: i64,
    ctime: i64,
    crtime: i64
}

impl Timestamps {
    pub fn new() -> Self {
        Self {
            atime: 0,
            mtime: 0,
            ctime: 0,
            crtime: 0
        }
    }
}

struct Bodyfile1 {
    md5: String,
    name: String,
    inode: u64,
    mode_as_string: String,
    uid: u64,
    gid: u64,
    size: u64,
    si_times: Timestamps,
    fn_times: Timestamps,
    is_allocated: bool,
    is_dir: bool,
    pub parent: MftReference,
    pub id: MftReference,
}

impl Bodyfile1 {
    pub fn new() -> Self {
        Self {
            md5: String::from("0"),
            name: String::new(),
            inode: 0,
            mode_as_string: String::from("0"),
            uid: 0,
            gid: 0,
            size: 0,
            si_times: Timestamps::new(),
            fn_times: Timestamps::new(),
            is_allocated: false,
            is_dir: false,
            parent: MftReference::new(0, 0),
            id: MftReference::new(0, 0)
        }
    }

    pub fn from(entry: mft::MftEntry) -> Self {
        let mut bfentry = Bodyfile1::new();
        bfentry.is_dir = entry.is_dir();
        bfentry.is_allocated = entry.is_allocated();
        bfentry.id.entry = entry.header.record_number;
        bfentry.id.sequence = entry.header.sequence;
        bfentry.inode = entry.header.record_number;

        match entry.find_best_name_attribute() {
            Some(filename_attribute) => {
                bfentry.name = filename_attribute.name.clone();
                bfentry.size = filename_attribute.logical_size;
                bfentry.fn_times.crtime = filename_attribute.created.timestamp();
                bfentry.fn_times.ctime = filename_attribute.modified.timestamp();
                bfentry.fn_times.mtime = filename_attribute.mft_modified.timestamp();
                bfentry.fn_times.atime = filename_attribute.accessed.timestamp();
                bfentry.parent = filename_attribute.parent;
            }
            None => ()
        }

        for attribute in entry.iter_attributes().filter_map(|attr| attr.ok()) {
            match attribute.data {
                MftAttributeContent::AttrX10(standard_info) => {
                    bfentry.si_times.crtime = standard_info.created.timestamp();
                    bfentry.si_times.ctime = standard_info.modified.timestamp();
                    bfentry.si_times.mtime = standard_info.mft_modified.timestamp();
                    bfentry.si_times.atime = standard_info.accessed.timestamp();
                }
                _ => ()
            }
        }
        bfentry
    }

    pub fn status_as_str(&self) -> &str {
        if self.is_allocated {
            ""
        } else {
            " (deleted)"
        }
    }

    pub fn format_si(&self, full_name: &str) -> String {
        format!("{}|{}{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            &self.md5,
            full_name,
            self.status_as_str(),
            &self.inode,
            &self.mode_as_string,
            &self.uid,
            &self.gid,
            &self.size,
            &self.si_times.atime,
            &self.si_times.mtime,
            &self.si_times.ctime,
            &self.si_times.crtime
        )
    }
    pub fn format_fn(&self, full_name: &str) -> String {
        format!("{}|{} ($FILE_NAME){}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            &self.md5,
            full_name,
            self.status_as_str(),
            &self.inode,
            &self.mode_as_string,
            &self.uid,
            &self.gid,
            &self.size,
            &self.fn_times.atime,
            &self.fn_times.mtime,
            &self.fn_times.ctime,
            &self.fn_times.crtime
        )
    }
}

fn full_path(mft: &HashMap<MftReference, Bodyfile1>, bf: &MftReference) -> String {
    match mft.get(bf) {
        Some(node) => {
            if node.parent.entry == 0 || &node.parent == bf{
                String::new()
            } else {
                //println!("searching parent of {}", &node.name);
                format!("{}/{}", full_path(mft, &node.parent), &node.name)
            }
        }
        None => String::new()
    }
}

fn main() -> Result<()> {
    // Change this to a path of your MFT sample.
    let mut filename = String::new();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("parses an $MFT file to bodyfile (stdout)");
        ap.refer(&mut filename).add_argument("mft_file", Store, "path to $MFT").required();
        ap.parse_args_or_exit();
    }

    let fp = PathBuf::from(&filename);
    if ! (fp.exists() && fp.is_file()) {
        return Err(anyhow::Error::msg(format!("File {} does not exist", &filename)));
    }
    let mut mft = HashMap::new();
    
    let mut parser = MftParser::from_path(fp).unwrap();
    for entry in parser.iter_entries() {
        match entry {
            Ok(e) =>  {
                let bf = Bodyfile1::from(e);
                mft.insert(bf.id.clone(), bf);
            }
            Err(err) => {
                return Err(anyhow::Error::msg(format!("{}", err)));
            }
        }
    }

    for e in mft.values() {
        let full_name = full_path(&mft, &e.id);
        println!("{}", e.format_si(&full_name));
        println!("{}", e.format_fn(&full_name));
    }

    Ok(())
}