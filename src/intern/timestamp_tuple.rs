
use mft::attribute::x10::StandardInfoAttr;
use mft::attribute::x30::FileNameAttr;

pub struct TimestampTuple {
    accessed: i64,
    mft_modified: i64,
    modified: i64,
    created: i64,
}

impl From<&FileNameAttr> for TimestampTuple {
    fn from(attr: &FileNameAttr) -> TimestampTuple {
        TimestampTuple {
            accessed: attr.accessed.timestamp(),
            mft_modified: attr.mft_modified.timestamp(),
            modified: attr.modified.timestamp(),
            created: attr.created.timestamp()
        }
    }
}


impl From<&StandardInfoAttr> for TimestampTuple {
    fn from(attr: &StandardInfoAttr) -> TimestampTuple {
        TimestampTuple {
            accessed: attr.accessed.timestamp(),
            mft_modified: attr.mft_modified.timestamp(),
            modified: attr.modified.timestamp(),
            created: attr.created.timestamp()
        }
    }
}

impl TimestampTuple {
    pub fn accessed(&self) -> i64 {self.accessed}
    pub fn mft_modified(&self) -> i64 {self.mft_modified}
    pub fn modified(&self) -> i64 {self.modified}
    pub fn created(&self) -> i64 {self.created}
}