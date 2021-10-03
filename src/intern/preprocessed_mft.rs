use anyhow::Result;
use likely_stable::unlikely;
use mft::attribute::x10::StandardInfoAttr;
use mft::attribute::x30::{FileNameAttr, FileNamespace};
use mft::attribute::{MftAttributeContent, MftAttributeType};
use mft::MftEntry;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use winstructs::ntfs::mft_reference::MftReference;

struct CompleteMftEntry {
    base_entry: MftReference,
    file_name_attribute: Option<FileNameAttr>,
    standard_info_attribute: Option<StandardInfoAttr>,
    full_path: RefCell<String>,
    is_allocated: bool,
}

impl CompleteMftEntry {
    pub fn from_base_entry(entry_reference: MftReference, entry: MftEntry) -> Self {
        let mut c = Self {
            base_entry: entry_reference,
            file_name_attribute: None,
            standard_info_attribute: None,
            full_path: RefCell::new(String::new()),
            is_allocated: entry.is_allocated(),
        };
        c.update_attributes(&entry);

        return c;
    }

    pub fn from_nonbase_entry(_entry_ref: MftReference, entry: MftEntry) -> Self {
        let mut c = Self {
            base_entry: entry.header.base_reference,
            file_name_attribute: None,
            standard_info_attribute: None,
            full_path: RefCell::new(String::new()),
            is_allocated: false,
        };
        c.add_nonbase_entry(entry);

        return c;
    }

    pub fn base_entry(&self) -> &MftReference {
        &self.base_entry
    }

    pub fn set_base_entry(&mut self, entry_ref: MftReference, entry: MftEntry) {
        assert_eq!(self.base_entry, entry_ref);
        self.update_attributes(&entry);
        self.is_allocated = entry.is_allocated();
    }

    fn add_nonbase_entry(&mut self, e: MftEntry) {
        self.update_attributes(&e);
    }

    fn update_attributes(&mut self, entry: &MftEntry) {
        let my_attribute_types = vec![
            MftAttributeType::FileName,
            MftAttributeType::StandardInformation,
        ];
        for attr_result in entry
            .iter_attributes_matching(Some(my_attribute_types))
            .filter_map(Result::ok)
        {
            match attr_result.data {
                MftAttributeContent::AttrX10(standard_info_attribute) => {
                    if self.standard_info_attribute.is_none() {
                        self.standard_info_attribute = Some(standard_info_attribute);
                    } else {
                        panic!("multiple standard information attributes found")
                    }
                }
                MftAttributeContent::AttrX30(file_name_attribute) => {
                    match self.file_name_attribute {
                        None => self.file_name_attribute = Some(file_name_attribute),
                        Some(ref mut name_attr) => match file_name_attribute.namespace {
                            FileNamespace::Win32AndDos => *name_attr = file_name_attribute,
                            FileNamespace::Win32 => {
                                if name_attr.namespace != FileNamespace::Win32AndDos {
                                    *name_attr = file_name_attribute
                                }
                            }
                            FileNamespace::POSIX => {
                                if name_attr.namespace == FileNamespace::DOS {
                                    *name_attr = file_name_attribute
                                }
                            }
                            FileNamespace::DOS => {}
                        },
                    }
                }
                _ => panic!("filter for iter_attributes_matching() isn't working"),
            }
        }
    }

    pub fn parent(&self) -> Option<MftReference> {
        match self.file_name_attribute {
            None => None,
            Some(ref fn_attr) => Some(fn_attr.parent),
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

            match self.file_name_attribute() {
                Some(name) => {
                    match self.parent() {
                        None => *self.full_path.borrow_mut() = name.name.to_string(),
                        Some(p) => {
                            // prevent endless recursion, mainly for $MFT entry 5 (which is the root directory)
                            assert_ne!(p, self.base_entry);

                            let parent_path = mft.get_full_path(&p);
                            let mut fp = self.full_path.borrow_mut();
                            *fp = parent_path;
                            fp.push('/');
                            fp.push_str(&name.name);
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
            Some(ref fn_attr) => fn_attr.logical_size,
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
                    fn_attr.accessed.timestamp(),
                    fn_attr.mft_modified.timestamp(),
                    fn_attr.modified.timestamp(),
                    fn_attr.created.timestamp(),
                ))
            }
            None => {
                None
            }
        }
    }
    pub fn format_si(&self, mft: &PreprocessedMft) -> String {
        match self.standard_info_attribute {
            Some(ref standard_info_attribute) => self.format(
                &self.get_full_path(mft),
                standard_info_attribute.accessed.timestamp(),
                standard_info_attribute.mft_modified.timestamp(),
                standard_info_attribute.modified.timestamp(),
                standard_info_attribute.created.timestamp(),
            ),
            None => panic!("missing standard information"),
        }
    }

    pub fn file_name_attribute(&self) -> &Option<FileNameAttr> {
        if self.file_name_attribute.is_none() {
            if self.is_allocated {
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
            } else {
                log::warn!(
                "no $FILE_NAME attribute found for $MFT entry {}-{}, but this is a deleted file",
                self.base_entry().entry,
                self.base_entry().sequence
            );
            }
        }
        return &self.file_name_attribute;
    }
}

pub struct PreprocessedMft {
    complete_entries: HashMap<MftReference, CompleteMftEntry>,
}

impl PreprocessedMft {
    pub fn new() -> Self {
        Self {
            complete_entries: HashMap::new(),
        }
    }
    pub fn add_entry(&mut self, entry: MftEntry) {
        let reference = MftReference::new(entry.header.record_number, entry.header.sequence);

        if PreprocessedMft::is_base_entry(&entry) {
            match self.complete_entries.get_mut(&reference) {
                Some(e) => e.set_base_entry(reference, entry),
                None => {
                    let ce = CompleteMftEntry::from_base_entry(reference, entry);
                    let _ = self.complete_entries.insert(reference, ce);
                }
            }
        } else
        /* if ! PreprocessedMft::is_base_entry(&entry) */
        {
            //
            // ignore unallocated nonbase entries
            //
            if entry.is_allocated() {
                let base_reference = entry.header.base_reference;
                match self.complete_entries.get_mut(&base_reference) {
                    Some(e) => {
                        e.add_nonbase_entry(entry);
                    }
                    None => {
                        let ce = CompleteMftEntry::from_nonbase_entry(reference, entry);
                        let _ = self.complete_entries.insert(base_reference, ce);
                    }
                }
            }
        }
    }

    pub fn is_base_entry(entry: &MftEntry) -> bool {
        entry.header.base_reference.entry == 0 && entry.header.base_reference.sequence == 0
    }

    pub fn get_full_path(&self, reference: &MftReference) -> String {
        match self.complete_entries.get(&reference) {
            None => format!("deleted_parent_{}_{}", reference.entry, reference.sequence),
            Some(entry) => entry.get_full_path(self),
        }
    }

    pub fn print_entries(&self) {
        let stdout = std::io::stdout();
        let mut stdout_lock = stdout.lock();

        for entry in self.complete_entries.values() {
            stdout_lock
                .write_all(entry.format_si(self).as_bytes())
                .unwrap();
            if let Some(fn_info) = entry.format_fn(self) {
                stdout_lock.write_all(fn_info.as_bytes()).unwrap();
            }
        }
    }
}
