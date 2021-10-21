use std::collections::hash_map::HashMap;
use usnjrnl::*;
use winstructs::ntfs::mft_reference::MftReference;

pub type KeyType = MftReference;
pub type ValueType = Vec<CommonUsnRecord>;

pub struct UsnJrnl {
    entries: HashMap<KeyType, ValueType>
}

impl Default for UsnJrnl {
    fn default() -> Self {
        Self {
            entries: HashMap::new()
        }
    }
}

impl UsnJrnl {

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[allow(clippy::should_implement_trait)]
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
                            let _ = entries.insert(data.FileReferenceNumber, vec![entry]);
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