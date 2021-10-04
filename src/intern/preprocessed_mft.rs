use mft::MftEntry;
use std::collections::HashMap;
use winstructs::ntfs::mft_reference::MftReference;
use crate::intern::CompleteMftEntry;

pub struct PreprocessedMft {
    complete_entries: HashMap<MftReference, CompleteMftEntry>,
    count: u64,
}

impl PreprocessedMft {
    pub fn new() -> Self {
        Self {
            complete_entries: HashMap::new(),
            count: 0,
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
                    self.count += 1;
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
                        self.count += 1;
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

    pub fn entries_count(&self) -> u64 {
        self.count
    }
    
    pub fn iter_entries<'a>(&'a self) -> Box<dyn Iterator<Item=String> + 'a>{
        Box::new(self.complete_entries
            .values()
            .map(move |c| c.bodyfile_lines(self))
            .flatten())
    }
}
