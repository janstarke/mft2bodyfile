use std::collections::hash_map::HashMap;
use crate::usnjrnl::*;
use winstructs::ntfs::mft_reference::MftReference;

pub struct UsnJrnl {
    entries: HashMap<MftReference, Vec<CommonUsnRecord>>
}

impl UsnJrnl {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new()
        }
    }
}

impl From<UsnJrnlReader> for UsnJrnl {
    fn from(reader: UsnJrnlReader) -> Self {
        let mut entries: HashMap<MftReference, Vec<CommonUsnRecord>> = HashMap::new();
        for entry in reader.into_iter() {
            match &entry.data {
                UsnRecordData::V2(data) => {
                    match entries.get_mut(&data.FileReferenceNumber) {
                        Some(ref mut v) => v.push(entry),
                        None => {
                            let _ = entries.insert(data.FileReferenceNumber.clone(), vec![entry]);
                        }
                    };
                }
            }
            
        }

        Self {
            entries
        }
    }
}