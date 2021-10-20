use from_bytes::*;
use from_bytes_derive::*;
use packed_struct::prelude::*;
use chrono::{DateTime, Utc};
use winstructs::timestamp::WinTimestamp;
use winstructs::ntfs::mft_reference::MftReference;

use super::usn_reader_error::*;
use super::usn_reason::*;

#[derive(Debug)]
pub struct CommonUsnRecord {
  pub header: UsnRecordCommonHeader,
  pub data: UsnRecordData,
}

fn utf16_from_slice(slice: &[u8], mut offset: usize, characters: usize) -> String {
  let mut name_chars = Vec::new();
  for _ in 0..characters {
    name_chars.push(slice[offset] as u16 | ((slice[offset + 1] as u16) << 8 as u8));
    offset += 2;
  }
  String::from_utf16_lossy(&name_chars[..])
}

impl CommonUsnRecord {
  pub fn from(data: &[u8], index: &mut usize) -> std::result::Result<Self, UsnReaderError> {
    let mut header = *UsnRecordCommonHeader::from_bytes(&data, *index)?;
    if header.RecordLength == 0 {
      /* looks like a cluster change, round index up to the next cluster */
      *index += 0x1000 - (*index & 0xfff);

      /* reread header at new address */
      header = *UsnRecordCommonHeader::from_bytes(&data, *index)?;

      if header.RecordLength == 0 {
        return Err(UsnReaderError::NoMoreData);
      }
    }

    let usn_data = match header.MajorVersion {
      2 => UsnRecordData::V2(UsnRecordV2::from(data, *index)?),
      3 => {
        return Err(UsnReaderError::SyntaxError(format!(
          "Version 3 records (ReFS only) are not supported yes"
        )));
      }
      4 => {
        return Err(UsnReaderError::SyntaxError(format!(
          "Version 4 records (ReFS only) are not supported yes"
        )));
      }
      version => {
        return Err(UsnReaderError::SyntaxError(format!(
          "invalid value for MajorVersion: {}",
          version
        )));
      }
    };

    Ok(Self {
      header,
      data: usn_data,
    })
  }
}

#[derive(Debug)]
pub enum UsnRecordData {
  V2(UsnRecordV2),
  //
  // this entry is not supported yet
  //V3(UsnRecordV3),

  // The user always receives one or more USN_RECORD_V4 records followed by one
  // USN_RECORD_V3 record.
  //
  // this entry is not supported yet
  /*
  V4(UsnRecordV4),
  */
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
struct BinaryUsnRecordV2 {
  pub FileReferenceNumber: u64,
  pub ParentFileReferenceNumber: u64,
  pub Usn: i64,
  pub TimeStamp: [u8;8],
  pub Reason: u32,
  pub SourceInfo: u32,
  pub SecurityId: u32,
  pub FileAttributes: u32,
  pub FileNameLength: u16,
  pub FileNameOffset: u16,
  pub FileName: u16,
}

#[derive(Debug)]
pub struct UsnRecordV2 {
  pub FileReferenceNumber: MftReference,
  pub ParentFileReferenceNumber: MftReference,
  pub Usn: i64,
  pub TimeStamp: DateTime<Utc>,
  pub Reason: UsnReason,
  pub SourceInfo: u32,
  pub SecurityId: u32,
  pub FileAttributes: u32,
  pub FileName: String,
}

impl UsnRecordV2 {
  fn from(data: &[u8], record_index: usize) -> std::result::Result<Self, UsnReaderError> {
    let record =
      BinaryUsnRecordV2::from_bytes(&data, record_index + UsnRecordCommonHeader::packed_size())?;

    let filename = utf16_from_slice(
      data,
      record_index + record.FileNameOffset as usize,
      (record.FileNameLength / 2) as usize,
    );

    let file_reference = MftReference::from(record.FileReferenceNumber);
    let parent_reference = MftReference::from(record.ParentFileReferenceNumber);
    let timestamp = WinTimestamp::new(&record.TimeStamp[..])
      .map_err(|_| UsnReaderError::FailedToReadWindowsTime(record.TimeStamp))?
      .to_datetime();
    Ok(Self {
      FileReferenceNumber: file_reference,
      ParentFileReferenceNumber: parent_reference,
      Usn: record.Usn,
      TimeStamp: timestamp,
      Reason: UsnReason::from(record.Reason),
      SourceInfo: record.SourceInfo,
      SecurityId: record.SecurityId,
      FileAttributes: record.FileAttributes,
      FileName: filename,
    })
  }
}

/*
#[derive(PackedStruct, Debug, StructFromBytes, PackedSize)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct UsnRecordV3 {
  pub FileReferenceNumber: [u8; 16],
  pub ParentFileReferenceNumber: [u8; 16],
  pub Usn: i64,
  pub TimeStamp: i64,
  #[packed_field(size_bytes="4")]
  pub Reason: u32,
  pub SourceInfo: u32,
  pub SecurityId: u32,
  pub FileAttributes: u32,
  pub FileNameLength: u16,
  pub FileNameOffset: u16,
  pub FileName: u16,
}
*/
/*
#[derive(PackedStruct, Debug, StructFromBytes, PackedSize)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct UsnRecordV4 {
  pub FileReferenceNumber: [u8; 16],
  pub ParentFileReferenceNumber: [u8; 16],
  pub Usn: i64,
  #[packed_field(size_bytes="4")]
  pub Reason: UsnReason,
  pub SourceInfo: u32,
  pub RemainingExtents: u32,
  pub NumberOfExtents: u16,
  pub ExtentSize: u16,
}

#[derive(PackedStruct, Debug, StructFromBytes, PackedSize)]
#[packed_struct(bit_numbering = "msb0", endian = "lsb")]
pub struct UsnRecordExtend {
  pub Offset: u64,
  pub Length: u64,
}
*/
