use mft::MftEntry;
use std::collections::HashMap;
use winstructs::ntfs::mft_reference::MftReference;
use crate::intern::CompleteMftEntry;
use usnjrnl::CommonUsnRecord;

pub struct ParentInfo {
    pub full_path: String,
    pub is_allocated: bool,
    pub reference: Option<MftReference>
}

#[derive(Default)]
pub struct PreprocessedMft {
    complete_entries: HashMap<MftReference, CompleteMftEntry>
}


impl PreprocessedMft {
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

    pub fn get_full_path(&self, reference: &MftReference) -> ParentInfo {
        if let Some(entry) = self.complete_entries.get(reference) {
            return ParentInfo {
                full_path: entry.get_full_path(self),
                is_allocated: entry.is_allocated(),
                reference: Some(*reference)
            }
        }

        // if the parent folder was already deleted, the sequence number is incremented
        let deleted_ref = MftReference::new(reference.entry, reference.sequence + 1);
        if let Some(entry) = self.complete_entries.get(&deleted_ref) {
            if ! entry.is_allocated() {
                return ParentInfo {
                    full_path: entry.get_full_path(self),
                    is_allocated: entry.is_allocated(),
                    reference: Some(deleted_ref)
                }
            }
        }

        ParentInfo {
            full_path: "/$OrphanFiles".to_string(),
            is_allocated: false,
            reference: None
        }
    }

    pub fn bodyfile_lines_count(&self) -> usize {
        self.complete_entries.values().map(|e| e.bodyfile_lines_count()).sum()
    }
    
    pub fn iter_entries<'a>(&'a self, usnjrnl_longflags: bool) -> Box<dyn Iterator<Item=String> + 'a>{
        Box::new(self.complete_entries
            .values()
            .flat_map(move |c| c.bodyfile_lines(self, usnjrnl_longflags)))
    }
}
