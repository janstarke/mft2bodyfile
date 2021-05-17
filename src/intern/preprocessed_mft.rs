
use crate::intern::*;

use mft::MftEntry;
use mft::attribute::{MftAttributeContent, MftAttributeType, MftAttribute};
use mft::attribute::x10::StandardInfoAttr;
use mft::attribute::x30::{FileNameAttr, FileNamespace};
use std::collections::HashMap;
use winstructs::ntfs::mft_reference::MftReference;
use anyhow::Result;
use std::io::Write;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PreprocessedMft {
  base_entries: HashMap<MftReference, PreprocessedMftEntry>,
  nonbase_entries: HashMap<MftReference, MftEntry>
}

impl PreprocessedMft {
  pub fn new() -> Rc<RefCell<Self>> {
      Rc::new(RefCell::new(Self {
        base_entries: HashMap::new(),
        nonbase_entries: HashMap::new()
      }))
  }

  pub fn insert_base_entry(&mut self, reference: MftReference, entry: PreprocessedMftEntry) {
      self.base_entries.insert(reference, entry);
  }

  pub fn insert_nonbase_entry(&mut self, reference: MftReference, entry: MftEntry) {
      self.nonbase_entries.insert(reference, entry);
  }

  pub fn is_base_entry(entry: &MftEntry) -> bool {
      entry.header.base_reference.entry==0 && entry.header.base_reference.sequence==0
  }

  pub fn update_bf_lines(&self) {
      for e in self.base_entries.values() {
          e.update_bf_line();
      }
  }

  pub fn get_full_path(&self, reference: &MftReference) -> String {
      match self.base_entries.get(reference) {
          None => panic!("did not find reference in $MFT"),
          Some(entry) => (*entry.get_full_path(self)).clone()
      }
  }

  pub fn get_nonbase_mft_entry(&self, reference: &MftReference) -> Option<&MftEntry> {
      self.nonbase_entries.get(reference)
  }
  
  pub fn nonbase_attributes_matching(&self,
                                      base_entry: &MftEntry,
                                      types: Vec<MftAttributeType>)
                                      -> Vec<MftAttribute> {
      match Bodyfile1Line::find_attribute_list(base_entry) {
        AttributeListResult::NoAttributeList => {
              vec![]
          }
          AttributeListResult::NonResidentAttributeList => {
              vec![]
          }
          AttributeListResult::ResidentAttributeList(a) => {
              //log::info!("found attribute list of len {}", a.entries.len());
              let mut attributes: Vec<MftAttribute> = Vec::new();
              for nonbase_entry in a.entries {
                  match self.get_nonbase_mft_entry(&nonbase_entry.segment_reference) {
                      None => continue,
                      Some(mft_entry) => {
                          let attr_iter = mft_entry.iter_attributes_matching(Some(types.clone()));
                          attributes.extend(attr_iter.filter_map(Result::ok));
                      }
                  }
              }
              attributes
          }
      }
  }

  pub fn find_filename(&self, entry: &MftEntry) -> Option<FileNameAttr> {
      let file_name_attributes: Vec<FileNameAttr> = entry
          .iter_attributes_matching(Some(vec![MftAttributeType::FileName]))
          .filter_map(Result::ok)
          .filter_map(|a| a.data.into_file_name()).chain(
              self.nonbase_attributes_matching(entry, vec![MftAttributeType::FileName]).into_iter()
              .filter_map(|a| a.data.into_file_name())
          )
          .collect();
      
      //log::info!("validating {} attributes", file_name_attributes.len());

          // Try to find a human-readable filename first
      let win32_filename = file_name_attributes
          .iter()
          .find(|a| [FileNamespace::Win32, FileNamespace::Win32AndDos].contains(&a.namespace));

      match win32_filename {
          Some(filename) => Some(filename.clone()),
          None => {
              // Try to take anything
              match file_name_attributes.iter().next() {
                  Some(filename) => Some(filename.clone()),
                  None => {
                      log::warn!("no $FILE_NAME attribute found for $MFT entry {}", entry.header.record_number);
                      /*
                      log::error!("the following attributes do exist:");
                      for a in entry.iter_attributes() {
                          log::error!("  >>> {:?}", a);
                      }
                      */
                      None
                  }
              }
          }
      }
  }

  pub fn find_standard_information(&self, entry: &MftEntry) -> StandardInfoAttr {
      for a in entry.iter_attributes()
                    .filter_map(|r| if let Ok(a)=r {Some(a)} else {None}) {
          match a.data {
              MftAttributeContent::AttrX10(si) => { return si; }

              MftAttributeContent::AttrX20(al) => {
                  match al.entries
                            .iter()
                            .find_map(|e| if e.attribute_type == MftAttributeType::StandardInformation as u32 {Some(e.segment_reference)} else {None}) {
                      Some(e) => {
                          let attribs = self.base_entries.get(&e).unwrap().mft_entry().iter_attributes();
                          for a2 in attribs.filter_map(|r| if let Ok(a)=r {Some(a)} else {None}) {
                              if let MftAttributeContent::AttrX10(si) = a2.data {
                                  return si;
                              }
                          }
                          panic!("mft is inconsistent: I did not found $STD_INFO where it ought to be");
                      }
                      None => { panic!("mft is invalid: mft entry has no $STD_INFO entry"); }
                  }
              }
              _ => ()
          }
      }

      eprintln!("{:?}", entry);
      for a in entry.iter_attributes()
      .filter_map(|r| if let Ok(a)=r {Some(a)} else {None}) {
          eprintln!("{:?}", a);
      }
      panic!("mft is invalid: mft entry has no $STD_INFO entry");
  }

  pub fn len(&self) -> usize {
      self.base_entries.len()
  }

  pub fn print_entries(&self) {
      let stdout = std::io::stdout();
      let mut stdout_lock = stdout.lock();

      for entry in self.base_entries.values().filter(|e| e.has_bf_line()) {
          stdout_lock.write_all(entry.format_si(self).as_bytes()).unwrap();
          if let Some(fn_info) = entry.format_fn(self) {
              stdout_lock.write_all(fn_info.as_bytes()).unwrap();
          }
      }
  }
}