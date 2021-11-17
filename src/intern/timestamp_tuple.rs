
use mft::attribute::x10::StandardInfoAttr;
use mft::attribute::x30::FileNameAttr;
use chrono::{DateTime, Utc};
use std::cmp;

pub struct TimestampTuple {
    accessed: i64,
    mft_modified: i64,
    modified: i64,
    created: i64,
}

impl TimestampTuple {
    fn win32_to_unix_timestamp(win32_ts: &DateTime<Utc>) -> i64 {
        /* any values below 0 cannot be used as unix timestamp */
        cmp::max(0, win32_ts.timestamp())
    }
}

impl From<&FileNameAttr> for TimestampTuple {
    fn from(attr: &FileNameAttr) -> TimestampTuple {
        TimestampTuple {
            accessed: Self::win32_to_unix_timestamp(&attr.accessed),
            mft_modified: Self::win32_to_unix_timestamp(&attr.mft_modified),
            modified: Self::win32_to_unix_timestamp(&attr.modified),
            created: Self::win32_to_unix_timestamp(&attr.created)
        }
    }
}


impl From<&StandardInfoAttr> for TimestampTuple {
    fn from(attr: &StandardInfoAttr) -> TimestampTuple {
        TimestampTuple {
            accessed: Self::win32_to_unix_timestamp(&attr.accessed),
            mft_modified: Self::win32_to_unix_timestamp(&attr.mft_modified),
            modified: Self::win32_to_unix_timestamp(&attr.modified),
            created: Self::win32_to_unix_timestamp(&attr.created)
        }
    }
}

impl TimestampTuple {
    pub fn accessed(&self) -> i64 {self.accessed}
    pub fn mft_modified(&self) -> i64 {self.mft_modified}
    pub fn modified(&self) -> i64 {self.modified}
    pub fn created(&self) -> i64 {self.created}
}