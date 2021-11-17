use crate::TimestampTuple;
use mft::attribute::header::MftAttributeHeader;
use mft::attribute::x30::{FileNameAttr, FileNamespace};
use winstructs::ntfs::mft_reference::MftReference;

pub struct FilenameInfo {
    filename: String,
    namespace: FileNamespace,
    timestamps: TimestampTuple,
    parent: MftReference,
    logical_size: u64,
    instance_id: u16
}

impl FilenameInfo {
    pub fn filename(&self) -> &String { &self.filename }
    pub fn timestamps(&self) -> &TimestampTuple { &self.timestamps }
    pub fn parent(&self) -> &MftReference { &self.parent }
    pub fn logical_size(&self) -> u64 { self.logical_size }
    pub fn instance_id(&self) -> u16 { self.instance_id }

    pub fn is_final(&self) -> bool {
        self.namespace == FileNamespace::Win32AndDos
    }

    pub fn from(attr: &FileNameAttr, header: &MftAttributeHeader) ->FilenameInfo {
        FilenameInfo {
            filename: attr.name.clone(),
            namespace: attr.namespace.clone(),
            timestamps: TimestampTuple::from(attr),
            parent: attr.parent,
            logical_size: attr.logical_size,
            instance_id: header.instance,
        }
    }

    pub fn update(&mut self, attr: &FileNameAttr, header: &MftAttributeHeader) {
        match attr.namespace {
            FileNamespace::Win32AndDos => self.do_update(attr, header),
            FileNamespace::Win32 => {
                if self.namespace != FileNamespace::Win32AndDos {
                    self.do_update(attr, header)
                }
            }
            FileNamespace::POSIX => {
                if self.namespace == FileNamespace::DOS {
                    self.do_update(attr, header)
                }
            }
            FileNamespace::DOS => {}
        }
    }

    fn do_update (&mut self, attr: &FileNameAttr, header: &MftAttributeHeader) {
        self.filename = attr.name.clone();
        self.namespace = attr.namespace.clone();
        self.timestamps = TimestampTuple::from(attr);
        self.parent = attr.parent;
        self.logical_size = attr.logical_size;
        self.instance_id = header.instance;
    }
}