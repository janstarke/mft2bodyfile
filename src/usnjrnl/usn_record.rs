use from_bytes::*;
use from_bytes_derive::*;
use packed_struct::prelude::*;
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
pub struct CommonUsnRecord {
  pub header: UsnRecordCommonHeader,
  pub data: UsnRecordData,
}

impl CommonUsnRecord {
  pub fn from(data: &[u8], index: usize) -> Result<Self> {
    let header = *UsnRecordCommonHeader::from_bytes(&data, index)?;

    let data = 
    match header.MajorVersion {
      2 => UsnRecordData::V2(*UsnRecordV2::from_bytes(&data, index + UsnRecordCommonHeader::packed_size())?),
      3 => UsnRecordData::V3(*UsnRecordV3::from_bytes(&data, index + UsnRecordCommonHeader::packed_size())?),
      4 => UsnRecordData::V4(*UsnRecordV4::from_bytes(&data, index + UsnRecordCommonHeader::packed_size())?),
      _ => return Err(Error::from(ErrorKind::InvalidData)),
    };

    Ok(
      Self {
        header,
        data
      }
    )
  }
}

#[derive(Debug)]
pub enum UsnRecordData {
  V2(UsnRecordV2),
  V3(UsnRecordV3),
  V4(UsnRecordV4)
}

#[derive(PackedStruct, Debug, StructFromBytes, PackedSize)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct UsnRecordCommonHeader {
  pub RecordLength: u32,
  pub MajorVersion: u16,
  pub MinorVersion: u16,
}

#[derive(PackedStruct, Debug, StructFromBytes, PackedSize)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct UsnRecordV2 {
  pub FileReferenceNumber: u64,
  pub ParentFileReferenceNumber: u64,
  pub Usn: i64,
  pub TimeStamp: i64,
  pub Reason: u32,
  pub SourceInfo: u32,
  pub SecurityId: u32,
  pub FileAttributes: u32,
  pub FileNameLength: u16,
  pub FileNameOffset: u16,
  pub FileName: u16,
}

#[derive(PackedStruct, Debug, StructFromBytes, PackedSize)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct UsnRecordV3 {
  pub FileReferenceNumber: [u8;16],
  pub ParentFileReferenceNumber: [u8;16],
  pub Usn: i64,
  pub TimeStamp: i64,
  pub Reason: u32,
  pub SourceInfo: u32,
  pub SecurityId: u32,
  pub FileAttributes: u32,
  pub FileNameLength: u16,
  pub FileNameOffset: u16,
  pub FileName: u16,
}

#[derive(PackedStruct, Debug, StructFromBytes, PackedSize)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct UsnRecordV4 {
  pub FileReferenceNumber: [u8;16],
  pub ParentFileReferenceNumber: [u8;16],
  pub Usn: i64,
  pub Reason: u32,
  pub SourceInfo: u32,
  pub RemainingExtents: u32,
  pub NumberOfExtents: u16,
  pub ExtentSize: u16,
}