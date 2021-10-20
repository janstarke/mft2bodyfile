use std::collections::hash_map::HashMap;
use usnjrnl::*;
use winstructs::ntfs::mft_reference::MftReference;

pub type KeyType = MftReference;
pub type ValueType = Vec<CommonUsnRecord>;

pub struct UsnJrnl {
    entries: HashMap<KeyType, ValueType>
}

impl UsnJrnl {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new()
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn into_iter(self) -> std::collections::hash_map::IntoIter<KeyType, ValueType> {
        self.entries.into_iter()
    }
}

impl From<UsnJrnlReader> for UsnJrnl {
    fn from(reader: UsnJrnlReader) -> Self {
        let mut entries: HashMap<KeyType, ValueType> = HashMap::new();
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