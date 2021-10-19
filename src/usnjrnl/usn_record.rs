use from_bytes::*;
use from_bytes_derive::*;
use packed_struct::prelude::*;
use std::fmt;

pub enum UsnReaderError {
  IO(std::io::Error),
  SyntaxError(String),
  NoMoreData
}

impl From<std::io::Error> for UsnReaderError {
  fn from(err: std::io::Error) -> Self {
    Self::IO(err)
  }
}

impl fmt::Display for UsnReaderError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::IO(io_error)     => write!(f, "IO Error: {}", io_error),
      Self::SyntaxError(err) => write!(f, "Syntax Error: {}", err),
      Self::NoMoreData       => write!(f, "no more data"),
    }
  }
}

#[derive(Debug)]
pub struct CommonUsnRecord {
  pub header: UsnRecordCommonHeader,
  pub data: UsnRecordData,
}

impl CommonUsnRecord {
  pub fn from(data: &[u8], index: usize) -> std::result::Result<Self, UsnReaderError> {
    let header = *UsnRecordCommonHeader::from_bytes(&data, index)?;
    
    if header.RecordLength == 0 {
      return Err(UsnReaderError::NoMoreData);
    }

    let data = 
    match header.MajorVersion {
      2 => UsnRecordData::V2(*UsnRecordV2::from_bytes(&data, index + UsnRecordCommonHeader::packed_size())?),
      3 => UsnRecordData::V3(*UsnRecordV3::from_bytes(&data, index + UsnRecordCommonHeader::packed_size())?),
      4 => UsnRecordData::V4(*UsnRecordV4::from_bytes(&data, index + UsnRecordCommonHeader::packed_size())?),
      version => {
          return Err(UsnReaderError::SyntaxError(format!("invalid value for MajorVersion: {}", version)));
      }
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

  /// The total length of a record, in bytes.
  /// 
  /// Because USN record is a variable size, the RecordLength member should be
  /// used when calculating the address of the next record in an output buffer,
  /// for example, a buffer that is returned from operations for the
  /// DeviceIoControl function that work with different USN record types.
  /// 
  /// For USN_RECORD_V4, the size in bytes of any change journal record is at
  /// most the size of the structure, plus (NumberOfExtents-1) times size of the
  /// USN_RECORD_EXTENT.
  pub RecordLength: u32,

  /// The major version number of the change journal software for this record.
  /// 
  /// For example, if the change journal software is version 4.0, the major version number is 4.
  /// 
  /// | Value | Description |
  /// |-|----|
  /// |2|The structure is a USN_RECORD_V2 structure and the remainder of the structure should be parsed using that layout.|
  /// |3|The structure is a USN_RECORD_V3 structure and the remainder of the structure should be parsed using that layout.|
  /// |4|The structure is a USN_RECORD_V4 structure and the remainder of the structure should be parsed using that layout.|
  pub MajorVersion: u16,

  /// The minor version number of the change journal software for this record. For example, if the change journal software
  /// is version 4.0, the minor version number is zero.
  pub MinorVersion: u16,
}

/// Contains the information for an update sequence number (USN) common header
/// which is common through USN_RECORD_V2, USN_RECORD_V3 and USN_RECORD_V4.
/// 
/// https://docs.microsoft.com/de-de/windows/win32/api/winioctl/ns-winioctl-usn_record_common_header
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