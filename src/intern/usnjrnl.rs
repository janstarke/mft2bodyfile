use crate::CommonUsnRecord;
use std::io::Result;
use std::fs::File;
use std::path::PathBuf;
use memmap::{Mmap, MmapOptions};

pub struct UsnJrnlReader {
    data: Mmap,
}

impl UsnJrnlReader {
    pub fn from(file_path: &PathBuf) -> Result<Self> {
        let file = File::open(file_path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        Ok(Self {
            data: mmap
        })
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> BorrowedUsrJrnlIterator {
        BorrowedUsrJrnlIterator {
            data: &self.data[..],
            current_offset: 0
        }
    }
}

impl IntoIterator for UsnJrnlReader {
    type Item = CommonUsnRecord;
    type IntoIter = OwnedUsrJrnlIterator;
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            data: self.data,
            current_offset: 0
        }
    }
}

fn next_from_data(data: &[u8], index: &mut usize) -> Option<CommonUsnRecord> {
    match CommonUsnRecord::from(data, *index) {
        Ok(record) => {
            *index += record.header.RecordLength as usize;
            Some(record)
        }

        Err(why) => {
            log::error!("error while parsing logfile: {}", why);
            None
        }
    }
}

pub struct BorrowedUsrJrnlIterator<'a> {
    data: &'a [u8],
    current_offset: usize,
}

impl<'a> Iterator for BorrowedUsrJrnlIterator<'a> {
    type Item = CommonUsnRecord;
    fn next(&mut self) -> Option<Self::Item> {
        next_from_data(self.data, &mut self.current_offset)
    }
}


pub struct OwnedUsrJrnlIterator {
    data: Mmap,
    current_offset: usize,
}

impl Iterator for OwnedUsrJrnlIterator {
    type Item = CommonUsnRecord;
    fn next(&mut self) -> Option<Self::Item> {
        next_from_data(&self.data[..], &mut self.current_offset)
    }
}