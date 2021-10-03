
use winstructs::ntfs::mft_reference::MftReference;
use std::fmt;

#[derive(Debug)]
pub enum NtfsError {
    DanglingNonbaseEntry(MftReference),
}

impl std::error::Error for NtfsError {}

impl fmt::Display for NtfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DanglingNonbaseEntry(r) => {
                write!(f, "missing base entry for nonbase entry {:?}", r)
            }
        }
    }
}