use mft::MftEntry;
use std::collections::HashMap;
use std::io::Write;
use winstructs::ntfs::mft_reference::MftReference;
use crate::intern::CompleteMftEntry;

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
