typedef struct {
    DWORD         RecordLength;
    WORD          MajorVersion;
    WORD          MinorVersion;
    DWORDLONG     FileReferenceNumber;
    DWORDLONG     ParentFileReferenceNumber;
    USN           Usn;
    LARGE_INTEGER TimeStamp;
    DWORD         Reason;
    DWORD         SourceInfo;
    DWORD         SecurityId;
    DWORD         FileAttributes;
    WORD          FileNameLength;
    WORD          FileNameOffset;
    WCHAR         FileName[1];
  }