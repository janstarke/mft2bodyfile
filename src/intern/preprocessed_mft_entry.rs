use crate::intern::*;

use mft::MftEntry;
use std::cell::{RefCell, Ref};
use likely_stable::{unlikely};
use std::rc::{Rc, Weak};

pub struct PreprocessedMftEntry {
  mft_entry: MftEntry,
  bf_line: RefCell<Option<Bodyfile1Line>>,
  full_path: RefCell<String>,
  mft: Weak<RefCell<PreprocessedMft>>
}

impl PreprocessedMftEntry {
  pub fn new(mft: &Rc<RefCell<PreprocessedMft>>, mft_entry: MftEntry) -> Self {
      Self {
          mft_entry,
          bf_line: RefCell::new(None),
          full_path: RefCell::new(String::new()),
          mft: Rc::downgrade(mft)
      }
  }

  pub fn mft_entry(&self) -> &MftEntry {
    &self.mft_entry
  }

  pub fn has_bf_line(&self) -> bool {
      self.bf_line.borrow().is_some()
  }

  pub fn mft(&self) -> Rc<RefCell<PreprocessedMft>> {
      self.mft.upgrade().unwrap()
  }

  pub fn update_bf_line(&self) {
      match Bodyfile1Line::from(&self.mft().borrow(), &self.mft_entry) {
          Some(bf_line) => *self.bf_line.borrow_mut() = Some(bf_line),
          None => ()
      }
  }

  pub fn get_full_path(&self, mft: &PreprocessedMft) -> Ref<String> {
      if unlikely(self.full_path.borrow().is_empty()) {
          match &self.bf_line.borrow().as_ref() {
              None => panic!("you did not create a bodyfile line"),
              Some(bf_line) => {
                  match &bf_line.parent {
                      None => *self.full_path.borrow_mut() = bf_line.name.clone(),
                      Some(p) => {
                          let mut fp = self.full_path.borrow_mut();
                          *fp = mft.get_full_path(p);
                          fp.push('/');
                          fp.push_str(&bf_line.name);
                      }
                  }
              }
          }
      }
      self.full_path.borrow()
  }

  pub fn format_fn(&self, mft: &PreprocessedMft) -> Option<String> {
      match self.bf_line.borrow().as_ref() {
          None => panic!("missing bf_line"),
          Some(bf_line) => bf_line.format_fn(& self.get_full_path(mft))
      }
  }
  pub fn format_si(&self, mft: &PreprocessedMft) -> String {
      match self.bf_line.borrow().as_ref() {
          None => panic!("missing bf_line"),
          Some(bf_line) => bf_line.format_si(& self.get_full_path(mft))
      }
  }
}
