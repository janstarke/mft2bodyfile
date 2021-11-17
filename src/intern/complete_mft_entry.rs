use crate::intern::{PreprocessedMft};
use crate::{FilenameInfo, TimestampTuple};
use anyhow::Result;
use likely_stable::unlikely;
use mft::attribute::{MftAttributeContent, MftAttributeType};
use mft::MftEntry;
use std::cell::RefCell;
use winstructs::ntfs::mft_reference::MftReference;
use usnjrnl::{CommonUsnRecord, UsnRecordData};
use bodyfile::Bodyfile3Line;
use num::ToPrimitive;

///
/// Represents the set of all $MFT entries that make up a files metadata.
/// The idea is to store only the minimum required data to generate
/// a bodyfile line, which would be
///
///  - the base reference (needed to print the `inode` number)
///  - the `$FILE_NAME` attribute. One file can have more than one `$FILE_NAME`
///    attribbutes, but we store only one of them. We choose the right attribute
///    using the following priority:
///    
///    1. `Win32AndDos`
///    2. `Win32`
///    3. `POSIX`
///    4. ´DOS`
///    If a file doesn't have a `$FILE_NAME` attribute, which may happen with already deleted files,
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
    deletion_status: RefCell<&'static str>,
    usnjrnl_records: Vec<CommonUsnRecord>,
    data_attributes: Vec<DataAttribute>,
    indexroot_attributes: Vec<u16>,
    is_directory: bool
}

pub struct DataAttribute {
    name: Option<String>,
    instance: u16
}

impl CompleteMftEntry {
    pub fn from_base_entry(entry_reference: MftReference, entry: MftEntry) -> Self {
        let mut c = Self {
            base_entry: entry_reference,
            file_name_attribute: None,
            standard_info_timestamps: None,
            full_path: RefCell::new(String::new()),
            is_allocated: entry.is_allocated(),
            usnjrnl_records: Vec::new(),
            deletion_status: RefCell::new(if entry.is_allocated() {""} else {" (deleted)"} ),
            data_attributes: Vec::new(),
            indexroot_attributes: Vec::new(),
            is_directory: entry.is_dir()
        };
        c.update_attributes(&entry);
        c
    }

    pub fn from_nonbase_entry(_entry_ref: MftReference, entry: MftEntry) -> Self {
        let mut c = Self {
            base_entry: entry.header.base_reference,
            file_name_attribute: None,
            standard_info_timestamps: None,
            full_path: RefCell::new(String::new()),
            is_allocated: false,
            usnjrnl_records: Vec::new(),
            deletion_status: RefCell::new(" (deleted)"),
            data_attributes: Vec::new(),
            indexroot_attributes: Vec::new(),
            is_directory: false
        };
        c.add_nonbase_entry(entry);
        c
    }

    pub fn from_usnjrnl_records(_entry_ref: MftReference, records: Vec<CommonUsnRecord>) -> Self {
        let mut records = records;
        records.sort_by(
            |a, b| a.data.timestamp().partial_cmp(b.data.timestamp()).unwrap());

        Self {
            base_entry: _entry_ref,
            file_name_attribute: None,
            standard_info_timestamps: None,
            full_path: RefCell::new(String::new()),
            is_allocated: false,
            usnjrnl_records: records,
            deletion_status: RefCell::new(" (deleted)"),
            data_attributes: Vec::new(),
            indexroot_attributes: Vec::new(),
            is_directory: false
        }
    }

    pub fn base_entry(&self) -> &MftReference {
        &self.base_entry
    }

    pub fn is_allocated(&self) -> bool {
        self.is_allocated
    }

    pub fn set_base_entry(&mut self, entry_ref: MftReference, entry: MftEntry) {
        assert_eq!(self.base_entry, entry_ref);

        self.update_attributes(&entry);
        self.is_allocated = entry.is_allocated();
        self.is_directory = entry.is_dir();
    }

    pub fn add_nonbase_entry(&mut self, e: MftEntry) {
        self.update_attributes(&e);
    }

    pub fn add_usnjrnl_records(&mut self, records: Vec<CommonUsnRecord>) {let mut records = records;
        records.sort_by(
            |a, b| a.data.timestamp().partial_cmp(b.data.timestamp()).unwrap());

        if self.usnjrnl_records.is_empty() {
            self.usnjrnl_records = records;
        } else {
            self.usnjrnl_records.extend(records);
        }
    }

    fn update_attributes(&mut self, entry: &MftEntry) {
        
        for attr_result in entry
            .iter_attributes_matching(Some(vec![
                MftAttributeType::StandardInformation,
                MftAttributeType::FileName,
                MftAttributeType::DATA,
                MftAttributeType::IndexRoot,
            ]))
            .filter_map(Result::ok)
        {
            if attr_result.header.type_code == MftAttributeType::IndexRoot {
                self.indexroot_attributes.push(attr_result.header.instance);
                continue;
            } else if attr_result.header.type_code == MftAttributeType::DATA {
                self.data_attributes.push(
                    DataAttribute {
                        name: attr_result.header.name_offset.and(Some(attr_result.header.name)),
                        instance: attr_result.header.instance
                    }
                );
                continue;
            }

            match attr_result.data {
                MftAttributeContent::AttrX10(standard_info_attribute) => {
                    if self.standard_info_timestamps.is_none() {
                        self.standard_info_timestamps =
                            Some(TimestampTuple::from(&standard_info_attribute));
                    } else {
                        panic!("multiple standard information attributes found")
                    }
                }

                MftAttributeContent::AttrX30(file_name_attribute) => {
                    match self.file_name_attribute {
                        None => {
                            self.file_name_attribute =
                                Some(FilenameInfo::from(&file_name_attribute, &attr_result.header))
                        }
                        Some(ref mut name_attr) => name_attr.update(&file_name_attribute, &attr_result.header),
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

    fn set_folder_name(&self, mft: &PreprocessedMft, parent: &MftReference, my_name: &str) {
        assert_ne!(parent, &self.base_entry);
        let mut fp = self.full_path.borrow_mut();

        let parent_info = mft.get_full_path(parent);

        *fp = parent_info.full_path;
        if ! &fp.ends_with('/') {
            fp.push('/');
        }
        fp.push_str(my_name);
    }

    pub fn get_full_path(&self, mft: &PreprocessedMft) -> String {
        if unlikely(self.full_path.borrow().is_empty()) {
            if self.base_entry.entry == 5
            /* matchs the root entry */
            {
                *self.full_path.borrow_mut() = String::from("/");
                return self.full_path.borrow().clone();
            }

            match self.filename_info() {
                Some(name) => {
                    match self.parent() {
                        None => *self.full_path.borrow_mut() = name.filename().clone(),
                        Some(p) => self.set_folder_name(mft, p, name.filename())
                    }
                }
                None => {
                    let my_name = match self.filename_from_usnjrnl() {
                        Some(name) => name.to_owned(),
                        None => format!(
                            "unnamed_{}_{}",
                            self.base_entry.entry, self.base_entry.sequence
                        )
                    };

                    match self.parent_from_usnjrnl() {
                        None => *self.full_path.borrow_mut() = my_name,
                        Some(p) => self.set_folder_name(mft, &p, &my_name)
                    };
                }
            }
        }
        self.full_path.borrow().to_string()
    }

    fn filename_from_usnjrnl(&self) -> Option<&str> {
        self.usnjrnl_records.last().map(|r| r.data.filename())
    }
    
    fn parent_from_usnjrnl(&self) -> Option<MftReference> {
        self.usnjrnl_records.last().and_then(|r| match &r.data {
                UsnRecordData::V2(data) => Some(data.ParentFileReferenceNumber),
                #[allow(unreachable_patterns)]
                _ => None
            }
        )
    }

    pub fn filesize(&self) -> u64 {
        match self.file_name_attribute {
            Some(ref fn_attr) => fn_attr.logical_size(),
            None => 0,
        }
    }

    fn format(
        &self,
        display_name: String,
        timestamps: &TimestampTuple,
        attribute_id: u32,
        instance_id: u16
    ) -> String {
        Bodyfile3Line::new()
            .with_owned_name(format!("{}{}",
                display_name,
                self.deletion_status.borrow()))
            .with_owned_inode(format!("{}-{}-{}",
                self.base_entry().entry.to_string(),
                attribute_id,
                instance_id))
            .with_size(self.filesize())
            .with_atime(timestamps.accessed())
            .with_mtime(timestamps.mft_modified())
            .with_ctime(timestamps.modified())
            .with_crtime(timestamps.created())
            .to_string()
    }

    fn format_fn(&self, mft: &PreprocessedMft) -> Option<String> {
        self.file_name_attribute.as_ref().and_then(|fn_attr|
            Some(self.format(
                    format!("{} ($FILE_NAME)", self.get_full_path(mft)),
                    fn_attr.timestamps(),
                    MftAttributeType::FileName.to_u32().unwrap(),
                    fn_attr.instance_id()
            )
        ))
    }

    fn format_si(&self, mft: &PreprocessedMft, 
        attribute_id: u32,
        instance_id: u16) -> Option<String> {
        self.standard_info_timestamps.as_ref().and_then(|si|
            Some(self.format(
                    self.get_full_path(mft),
                    si,
                    attribute_id,
                    instance_id
            )
        ))
    }


    /// returns the filename stored in the `$MFT`, if any, or None
    fn mft_filename(&self) -> Option<&String> {
        match &self.file_name_attribute {
            Some(fni) => Some(fni.filename()),
            None => None
        }
    }

    fn format_usnjrnl(&self, mft: &PreprocessedMft, record: &CommonUsnRecord, usnjrnl_longflags: bool) -> String {
        match &record.data {
            UsnRecordData::V2(data) => {
                let filename_info = match self.mft_filename() {
                    None => format!(" filename={}", data.FileName),
                    Some(f) => if f != &data.FileName {
                        format!(" filename={}", data.FileName)
                    } else {
                        "".to_owned()
                    }
                };

                let reason_info = if usnjrnl_longflags {
                    format!(" reason={:+}", data.Reason)
                } else {
                    format!(" reason={}", data.Reason)
                };

                let parent_info = mft.get_full_path(&data.ParentFileReferenceNumber);
                let parent_info = match &parent_info.reference {
                    Some(parent_ref) => {
                        if parent_ref == &data.ParentFileReferenceNumber {
                            "".to_owned()
                        } else if !parent_info.is_allocated &&
                            parent_ref == &MftReference::new(
                                data.ParentFileReferenceNumber.entry,
                                data.ParentFileReferenceNumber.sequence+1) {
                            "".to_owned()
                        } else {
                            format!(" parent={}-{}/{}-{}/'{}'",
                                parent_ref.entry,
                                parent_ref.sequence,
                                data.ParentFileReferenceNumber.entry,
                                data.ParentFileReferenceNumber.sequence,
                                parent_info.full_path)
                        }
                    }
                    None => format!(" parent='{}'", parent_info.full_path)
                };

                let display_name = format!("{} ($UsnJrnl{}{}{})",
                        self.get_full_path(mft),
                        filename_info,
                        parent_info,
                        reason_info);
                let timestamp = data.TimeStamp.timestamp();
                Bodyfile3Line::new()
                    .with_owned_name(display_name)
                    .with_atime(timestamp)
                    .to_string()
            }
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
            log::error!(
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

    pub fn bodyfile_lines(&self, mft: &PreprocessedMft, usnjrnl_longflags: bool) -> BodyfileLines {
        let attribute_id = if self.is_directory {
            MftAttributeType::IndexRoot.to_u32().unwrap()
        } else {
            MftAttributeType::DATA.to_u32().unwrap()
        };

        //FIXME: obtain real instance id
        let instance_id = 1;

        BodyfileLines {
           

            standard_info: self.format_si(mft, attribute_id, instance_id),
            filename_info: self.format_fn(mft),
            usnjrnl_records: self.usnjrnl_records
                                    .iter()
                                    .map(|r| self.format_usnjrnl(mft, r, usnjrnl_longflags))
                                    .collect()
        }
    }

    pub fn bodyfile_lines_count(&self) -> usize {
        (match &self.standard_info_timestamps {
            Some(_) => 1,
            None    => 0,
        }
        +
        match &self.file_name_attribute {
            Some(_) => 1,
            None    => 0,
        }
        +
        self.usnjrnl_records.len())
    }
}

pub struct BodyfileLines {
    standard_info: Option<String>,
    filename_info: Option<String>,
    usnjrnl_records: Vec<String>
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
        self.usnjrnl_records.pop()
    }
}
