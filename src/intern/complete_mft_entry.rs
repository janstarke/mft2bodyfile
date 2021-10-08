use anyhow::Result;
use likely_stable::unlikely;
use mft::attribute::{MftAttributeContent, MftAttributeType};
use std::cell::RefCell;
use mft::MftEntry;
use winstructs::ntfs::mft_reference::MftReference;
use crate::intern::PreprocessedMft;
use crate::{TimestampTuple, FilenameInfo};

///
/// Represents the set of all $MFT entries that make up a files metadata.
/// The idea is to store only the minimum required data to generate 
/// a bodyfile line, which would be 
/// 
///  - the base reference (needed to print the `inode` number)
///  - the `$FILENAME` attribute. One file can have more than one `$FILENAME`
///    attribbutes, but we store only one of them. We choose the right attribute
///    using the following priority:
///    
///    1. `Win32AndDos`
///    2. `Win32`
///    3. `POSIX`
///    4. ´DOS`
///    If a file doesn't have a `$FILENAME` attribute, which may happen with already deleted files,
///    then a filename is being generated, but *not* stored in `file_name_attribute`
/// 
///    This attribute is required to display the filename, but also contains four timestamps,
///    which are being displayed as well.
/// 
///  - the `$STANDARD_INFORMATION` attribute. This attribute contains four timestamps.
pub struct CompleteMftEntry {
    base_entry: MftReference,
    file_name_attribute: Option<FilenameInfo>,
    standard_info_timestamps: Option<TimestampTuple>,
    full_path: RefCell<String>,
    is_allocated: bool,
}

impl CompleteMftEntry {
    pub fn from_base_entry(entry_reference: MftReference, entry: MftEntry) -> Self {
        let mut c = Self {
            base_entry: entry_reference,
            file_name_attribute: None,
            standard_info_timestamps: None,
            full_path: RefCell::new(String::new()),
            is_allocated: entry.is_allocated(),
        };
        c.update_attributes(&entry, vec![MftAttributeType::StandardInformation,
                                         MftAttributeType::FileName]);
        c
    }

    pub fn from_nonbase_entry(_entry_ref: MftReference, entry: MftEntry) -> Self {
        let mut c = Self {
            base_entry: entry.header.base_reference,
            file_name_attribute: None,
            standard_info_timestamps: None,
            full_path: RefCell::new(String::new()),
            is_allocated: false,
        };
        c.add_nonbase_entry(entry);
        c
    }

    pub fn base_entry(&self) -> &MftReference {
        &self.base_entry
    }

    pub fn set_base_entry(&mut self, entry_ref: MftReference, entry: MftEntry) {
        assert_eq!(self.base_entry, entry_ref);

        self.update_attributes(&entry, vec![MftAttributeType::StandardInformation,
                                            MftAttributeType::FileName]);
        self.is_allocated = entry.is_allocated();
    }

    pub fn add_nonbase_entry(&mut self, e: MftEntry) {
        self.update_attributes(&e, vec![MftAttributeType::FileName]);
    }

    fn update_attributes(&mut self, entry: &MftEntry,
                                    attribute_types: Vec<MftAttributeType>) {
        /*
            do nothing if we already have all attributes
        */
        if self.standard_info_timestamps.is_some() {
            if let Some(filename_info) = &self.file_name_attribute {
                if filename_info.is_final() {
                    return;
                }
            }
        }

        let a = entry.iter_attributes_matching(Some(attribute_types));
        let b = a.filter_map(Result::ok);
        for attr_result in b
        {
            match attr_result.data {
                MftAttributeContent::AttrX10(standard_info_attribute) => {
                    if self.standard_info_timestamps.is_none() {
                        self.standard_info_timestamps = Some(TimestampTuple::from(&standard_info_attribute));
                    } else {
                        panic!("multiple standard information attributes found")
                    }
                }
                /*
                MftAttributeContent::AttrX20(attribute_list) => {
                    for attr_entry in attribute_list.entries {
                        if attr_entry.attribute_type == 0x10
                    }
                }*/
                MftAttributeContent::AttrX30(file_name_attribute) => {
                    match self.file_name_attribute {
                        None => self.file_name_attribute = Some(FilenameInfo::from(&file_name_attribute)),
                        Some(ref mut name_attr) => name_attr.update(&file_name_attribute),
                    }

                    if let Some(file_name_attribute) = &self.file_name_attribute {
                        if file_name_attribute.is_final() {
                            return;
                        }
                    }
                }
                _ => panic!("filter for iter_attributes_matching() isn't working"),
            }
        }
    }

    pub fn parent(&self) -> Option<&MftReference> {
        match self.file_name_attribute {
            None => None,
            Some(ref fn_attr) => Some(fn_attr.parent()),
        }
    }

    pub fn get_full_path(&self, mft: &PreprocessedMft) -> String {
        if unlikely(self.full_path.borrow().is_empty()) {
            if self.base_entry.entry == 5
            /* matchs the root entry */
            {
                *self.full_path.borrow_mut() = String::from("");
                return self.full_path.borrow().clone();
            }

            match self.filename_info() {
                Some(name) => {
                    match self.parent() {
                        None => *self.full_path.borrow_mut() = name.filename().clone(),
                        Some(p) => {
                            // prevent endless recursion, mainly for $MFT entry 5 (which is the root directory)
                            assert_ne!(p, &self.base_entry);

                            let parent_path = mft.get_full_path(p);
                            let mut fp = self.full_path.borrow_mut();
                            *fp = parent_path;
                            fp.push('/');
                            fp.push_str(name.filename());
                        }
                    }
                }
                None => {
                    *self.full_path.borrow_mut() = format!(
                        "unnamed_{}_{}",
                        self.base_entry.entry, self.base_entry.sequence
                    );
                }
            }
        }
        self.full_path.borrow().to_string()
    }

    pub fn filesize(&self) -> u64 {
        match self.file_name_attribute {
            Some(ref fn_attr) => fn_attr.logical_size(),
            None => 0,
        }
    }

    fn format(
        &self,
        display_name: &str,
        atime: i64,
        mtime: i64,
        ctime: i64,
        crtime: i64,
    ) -> String {
        let mode = String::from("0");
        let uid = String::from("0");
        let gid = String::from("0");
        let status = if self.is_allocated { "" } else { " (deleted)" };
        let filesize = self.filesize();
        format!(
            "0|{}{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            display_name,
            status,
            &self.base_entry().entry.to_string(),
            mode,
            uid,
            gid,
            filesize,
            atime,
            mtime,
            ctime,
            crtime
        )
    }

    pub fn format_fn(&self, mft: &PreprocessedMft) -> Option<String> {
        match self.file_name_attribute {
            Some(ref fn_attr) => {
                let display_name = format!("{} ($FILENAME)", self.get_full_path(mft));
                Some(self.format(
                    &display_name,
                    fn_attr.timestamps().accessed(),
                    fn_attr.timestamps().mft_modified(),
                    fn_attr.timestamps().modified(),
                    fn_attr.timestamps().created(),
                ))
            }
            None => {
                None
            }
        }
    }
    pub fn format_si(&self, mft: &PreprocessedMft) -> String {
        match self.standard_info_timestamps {
            Some(ref standard_info_timestamps) => self.format(
                &self.get_full_path(mft),
                standard_info_timestamps.accessed(),
                standard_info_timestamps.mft_modified(),
                standard_info_timestamps.modified(),
                standard_info_timestamps.created(),
            ),
            None => panic!("missing standard information"),
        }
    }

    pub fn filename_info(&self) -> &Option<FilenameInfo> {
        if self.file_name_attribute.is_none() && self.is_allocated {
            #[cfg(debug_assertions)]
            panic!(
                "no $FILE_NAME attribute found for $MFT entry {}-{}",
                self.base_entry().entry,
                self.base_entry().sequence
            );

            #[cfg(not(debug_assertions))]
            log::fatal!(
            "no $FILE_NAME attribute found for $MFT entry {}-{}. This is fatal because this is not a deleted file",
            self.base_entry().entry,
            self.base_entry().sequence
            );
        } /*else {
                log::warn!(
                "no $FILE_NAME attribute found for $MFT entry {}-{}, but this is a deleted file",
                self.base_entry().entry,
                self.base_entry().sequence
            );
            }*/
        &self.file_name_attribute
    }

    pub fn bodyfile_lines(&self, mft: &PreprocessedMft) -> BodyfileLines {
        BodyfileLines {
            standard_info: Some(self.format_si(mft)),
            filename_info: self.format_fn(mft)
        }
    }
}

pub struct BodyfileLines {
    standard_info: Option<String>,
    filename_info: Option<String>
}

impl Iterator for BodyfileLines {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        if self.standard_info.is_some() {
            return self.standard_info.take();
        }
        if self.filename_info.is_some() {
            return self.filename_info.take();
        }
        None
    }
}
