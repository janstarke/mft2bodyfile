use std::{collections::hash_map::HashMap};
use indicatif::ProgressBar;
use usnjrnl::*;
use winstructs::ntfs::mft_reference::MftReference;

pub type KeyType = MftReference;
pub type ValueType = Vec<CommonUsnRecord>;

#[derive(Default)]
pub struct UsnJrnl {
    entries: HashMap<KeyType, ValueType>
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

    pub fn from(reader: UsnJrnlReader, bar: ProgressBar) -> Self {
        let mut entries: HashMap<KeyType, ValueType> = HashMap::new();
        for entry in reader.into_iter() {
            match entry {
                Err(_) => { /* ignore that error for now */ }
                Ok(e) => {
                    match &e.data {
                        UsnRecordData::V2(data) => {
                            match entries.get_mut(&data.FileReferenceNumber) {
                                Some(ref mut v) => v.push(e),
                                None => {
                                    let _ = entries.insert(data.FileReferenceNumber, vec![e]);
                                    bar.inc(1);
                                }
                            };
                        }
                    }
                }
            }
        }
        bar.finish_at_current_pos();

        Self {
            entries
        }
    }
}