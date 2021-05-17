use crate::intern::*;

use mft::attribute::{MftAttributeContent, MftAttributeType};
use mft::attribute::x20::AttributeListAttr;
use winstructs::ntfs::mft_reference::MftReference;
use anyhow::Result;

pub struct Bodyfile1Line {
  pub name: String,
  pub parent: Option<MftReference>,
  pub id: MftReference,
  si_information: String,
  fn_information: Option<String>,
}

pub enum AttributeListResult {
    NoAttributeList,
    NonResidentAttributeList,
    ResidentAttributeList(AttributeListAttr)
}

impl Bodyfile1Line {
  pub fn from(mft: &PreprocessedMft, entry: &mft::MftEntry) -> Option<Self> {
      let mut si_information = String::with_capacity(70);

      let mode_uid_gid = "|0|0|0|";
      
      let status = if entry.is_allocated() { "" } else { " (deleted)" };
      let filename_attribute = mft.find_filename(entry).or_else(|| {
          log::warn!("didn't find any $FILE_NAME attribute");
          None
      });

      let filename = match filename_attribute.as_ref() {
          Some(fn_attr) => fn_attr.name.clone(),
          None          => format!("unnamed_{}_{} (missing $FILE_NAME in $MFT)", entry.header.record_number, entry.header.sequence)
      };
      let filesize = match filename_attribute.as_ref() {
          Some(fn_attr) => fn_attr.logical_size.to_string(),
          None          => String::from("0")
      };

      si_information.push_str(status);
      si_information.push('|');
      si_information.push_str(&entry.header.record_number.to_string());
      si_information.push_str(mode_uid_gid);
      si_information.push_str(&filesize);
      si_information.push('|');

      let fn_information = match filename_attribute.as_ref() {
          None => None,
          Some(fn_attr) => {
              let mut fn_information = String::with_capacity(70);
              fn_information.push_str(&si_information);
              fn_information.push_str(&fn_attr.accessed.timestamp().to_string());
              fn_information.push('|');
              fn_information.push_str(&fn_attr.mft_modified.timestamp().to_string());
              fn_information.push('|');
              fn_information.push_str(&fn_attr.modified.timestamp().to_string());
              fn_information.push('|');
              fn_information.push_str(&fn_attr.created.timestamp().to_string());
              fn_information.push('\n');
              Some(fn_information)
          }
      };

      let standard_info = mft.find_standard_information(&entry);
      si_information.push_str(&standard_info.accessed.timestamp().to_string());
      si_information.push('|');
      si_information.push_str(&standard_info.mft_modified.timestamp().to_string());
      si_information.push('|');
      si_information.push_str(&standard_info.modified.timestamp().to_string());
      si_information.push('|');
      si_information.push_str(&standard_info.created.timestamp().to_string());
      si_information.push('\n');

      let parent = 
      match filename_attribute.as_ref() {
          None => None,
          Some(fn_attr) => {
              if fn_attr.parent.entry == entry.header.record_number {
                  log::warn!("this entry has no parent");
                  None
              } else {
                  Some(fn_attr.parent)
              }
          }
      };

      Some(Bodyfile1Line {
          name: filename,
          parent,
          id: MftReference::new(entry.header.record_number, entry.header.sequence),
          fn_information,
          si_information,
      })
  }

  pub fn format_si(&self, full_name: &str) -> String {
      format!("0|{}{}",
          full_name,
          self.si_information
      )
  }
  pub fn format_fn(&self, full_name: &str) -> Option<String> {
      self.fn_information.as_ref().and_then(|fn_info| {
          Some(format!("0|{} ($FILE_NAME){}",
          full_name,
          &fn_info ))
      })
  }

  pub fn find_attribute_list(entry: &mft::MftEntry) -> AttributeListResult {
      match entry.iter_attributes_matching(Some(vec!(MftAttributeType::AttributeList)))
           .find_map(Result::ok) {
               None => AttributeListResult::NoAttributeList,
               Some(r) => match r.data {
                MftAttributeContent::AttrX20(a) => AttributeListResult::ResidentAttributeList(a),
                _ => {
                    log::warn!("$MFT entry {} has AttributeList without entries", entry.header.record_number);
                    log::warn!("{:?}", r.header);
                    AttributeListResult::NonResidentAttributeList
                }
            }
        }
  }
}