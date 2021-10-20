use mft::MftEntry;
use std::collections::HashMap;
use winstructs::ntfs::mft_reference::MftReference;
use crate::intern::CompleteMftEntry;
use usnjrnl::CommonUsnRecord;

pub struct PreprocessedMft {
    complete_entries: HashMap<MftReference, CompleteMftEntry>
}

impl PreprocessedMft {
    pub fn new() -> Self {
        Self {
            complete_entries: HashMap::new()
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
        } else if entry.is_allocated() { /* && ! PreprocessedMft::is_base_entry(&entry) */
            //
            // ignore unallocated nonbase entries
            //
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

    pub fn add_usnjrnl_records(&mut self, reference: MftReference, records: Vec<CommonUsnRecord>) {
        match self.complete_entries.get_mut(&reference) {
            Some(e) => {
                e.add_usnjrnl_records(records);
            }
            None => {
                let ce = CompleteMftEntry::from_usnjrnl_records(reference, records);
                let _ = self.complete_entries.insert(reference, ce);
            }
        }
    }

    pub fn is_base_entry(entry: &MftEntry) -> bool {
        entry.header.base_reference.entry == 0 && entry.header.base_reference.sequence == 0
    }

    pub fn get_full_path(&self, reference: &MftReference) -> String {
        match self.complete_entries.get(reference) {
            None => format!("deleted_parent_{}_{}", reference.entry, reference.sequence),
            Some(entry) => entry.get_full_path(self),
        }
    }

    pub fn entries_count(&self) -> usize {
        self.complete_entries.len()
    }

    pub fn bodyfile_lines_count(&self) -> usize {
        self.complete_entries.values().map(|e| e.bodyfile_lines_count()).sum()
    }
    
    pub fn iter_entries<'a>(&'a self, usnjrnl_longflags: bool) -> Box<dyn Iterator<Item=String> + 'a>{
        Box::new(self.complete_entries
            .values()
            .map(move |c| c.bodyfile_lines(self, usnjrnl_longflags))
            .flatten())
    }
}
