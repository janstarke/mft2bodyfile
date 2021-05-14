use mft::{MftParser, MftEntry};
use mft::attribute::{MftAttributeContent, MftAttributeType, MftAttribute};
use mft::attribute::x10::StandardInfoAttr;
use mft::attribute::x20::AttributeListAttr;
use mft::attribute::x30::{FileNameAttr, FileNamespace};
//use mft::attribute::x20::AttributeListAttr;
use std::path::PathBuf;
use std::collections::HashMap;
use winstructs::ntfs::mft_reference::MftReference;
use argparse::{ArgumentParser, Store};
use anyhow::Result;
use std::io::Write;
use std::cell::{RefCell, Ref};
use likely_stable::{unlikely};
use std::rc::{Rc, Weak};
use simplelog::{TermLogger, LevelFilter, Config, TerminalMode, ColorChoice};

struct PreprocessedMftEntry {
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

    pub fn format_fn(&self, mft: &PreprocessedMft) -> String {
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

struct PreprocessedMft {
    preprocessed_mft: HashMap<MftReference, PreprocessedMftEntry>
}

impl PreprocessedMft {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            preprocessed_mft: HashMap::new()
        }))
    }

    pub fn insert(&mut self, reference: MftReference, entry: PreprocessedMftEntry) {
        self.preprocessed_mft.insert(reference, entry);
    }

    pub fn is_base_record(entry: &MftEntry) -> bool {
        entry.header.base_reference.entry==0 && entry.header.base_reference.sequence==0
    }

    pub fn update_bf_lines(&self) {
        for e in self.preprocessed_mft.values().filter(|e| Self::is_base_record(&e.mft_entry)) {
            e.update_bf_line();
        }
    }

    pub fn get_full_path(&self, reference: &MftReference) -> String {
        match self.preprocessed_mft.get(reference) {
            None => panic!("did not find reference in $MFT"),
            Some(entry) => (*entry.get_full_path(self)).clone()
        }
    }

    pub fn get_ppentry<'a> (&'a self, reference: &MftReference) -> Option<&'a PreprocessedMftEntry> {
        self.preprocessed_mft.get(reference).or_else(|| {
            log::warn!("unable to resolve $MFT reference: {:?}", reference);
            None
        })
    }

    pub fn get_mft_entry(&self, reference: &MftReference) -> Option<&MftEntry> {
        self.get_ppentry(reference).and_then(|e| Some(&e.mft_entry))
    }
    
    pub fn nonbase_attributes_matching(&self,
                                        base_entry: &MftEntry,
                                        types: Vec<MftAttributeType>)
                                        -> Vec<MftAttribute> {
        match Bodyfile1Line::find_attribute_list(base_entry) {
            None => {
                //log::warn!("no attribute list found");
                vec![]
            }
            Some(a) => {
                //log::info!("found attribute list of len {}", a.entries.len());
                let mut attributes: Vec<MftAttribute> = Vec::new();
                for nonbase_entry in a.entries {
                    match self.get_mft_entry(&nonbase_entry.segment_reference) {
                        None => continue,
                        Some(mft_entry) => {
                            let attr_iter = mft_entry.iter_attributes_matching(Some(types.clone()));
                            attributes.extend(attr_iter.filter_map(Result::ok));
                        }
                    }
                }
                attributes
            }
        }
    }

    fn find_filename(&self, entry: &MftEntry) -> Option<FileNameAttr> {
        let file_name_attributes: Vec<FileNameAttr> = entry
            .iter_attributes_matching(Some(vec![MftAttributeType::FileName]))
            .filter_map(Result::ok)
            .filter_map(|a| a.data.into_file_name()).chain(
                self.nonbase_attributes_matching(entry, vec![MftAttributeType::FileName]).into_iter()
                .filter_map(|a| a.data.into_file_name())
            )
            .collect();
        
        //log::info!("validating {} attributes", file_name_attributes.len());

            // Try to find a human-readable filename first
        let win32_filename = file_name_attributes
            .iter()
            .find(|a| [FileNamespace::Win32, FileNamespace::Win32AndDos].contains(&a.namespace));

        match win32_filename {
            Some(filename) => Some(filename.clone()),
            None => {
                // Try to take anything
                match file_name_attributes.iter().next() {
                    Some(filename) => Some(filename.clone()),
                    None => {
                        log::warn!("no $FILE_NAME attribute found for $MFT entry {}", entry.header.record_number);
                        /*
                        log::error!("the following attributes do exist:");
                        for a in entry.iter_attributes() {
                            log::error!("  >>> {:?}", a);
                        }
                        */
                        None
                    }
                }
            }
        }
    }

    fn find_standard_information(&self, entry: &MftEntry) -> StandardInfoAttr {
        for a in entry.iter_attributes()
                      .filter_map(|r| if let Ok(a)=r {Some(a)} else {None}) {
            match a.data {
                MftAttributeContent::AttrX10(si) => { return si; }

                MftAttributeContent::AttrX20(al) => {
                    match al.entries
                              .iter()
                              .find_map(|e| if e.attribute_type == MftAttributeType::StandardInformation as u32 {Some(e.segment_reference)} else {None}) {
                        Some(e) => {
                            let attribs = self.preprocessed_mft.get(&e).unwrap().mft_entry.iter_attributes();
                            for a2 in attribs.filter_map(|r| if let Ok(a)=r {Some(a)} else {None}) {
                                if let MftAttributeContent::AttrX10(si) = a2.data {
                                    return si;
                                }
                            }
                            panic!("mft is inconsistent: I did not found $STD_INFO where it ought to be");
                        }
                        None => { panic!("mft is invalid: mft entry has no $STD_INFO entry"); }
                    }
                }
                _ => ()
            }
        }

        eprintln!("{:?}", entry);
        for a in entry.iter_attributes()
        .filter_map(|r| if let Ok(a)=r {Some(a)} else {None}) {
            eprintln!("{:?}", a);
        }
        panic!("mft is invalid: mft entry has no $STD_INFO entry");
    }

    pub fn len(&self) -> usize {
        self.preprocessed_mft.len()
    }

    pub fn print_entries(&self) {
        let stdout = std::io::stdout();
        let mut stdout_lock = stdout.lock();

        for entry in self.preprocessed_mft.values().filter(|e| e.has_bf_line()) {
            stdout_lock.write_all(entry.format_fn(self).as_bytes()).unwrap();
            stdout_lock.write_all(entry.format_si(self).as_bytes()).unwrap();
        }
    }
}


struct Bodyfile1Line {
    pub name: String,
    pub parent: Option<MftReference>,
    pub id: MftReference,
    si_information: String,
    fn_information: String,
}

impl Bodyfile1Line {
    pub fn from(mft: &PreprocessedMft, entry: &mft::MftEntry) -> Option<Self> {
        let mut si_information = String::with_capacity(70);
        let mut fn_information = String::with_capacity(70);

        let mode_uid_gid = "|0|0|0|";
        
        let status = if entry.is_allocated() { "" } else { " (deleted)" };
        let filename_attribute = match mft.find_filename(entry) {
            Some(fn_attr) => fn_attr,
            None     => {
                log::warn!("didn't find any $FILE_NAME attribute");
                return None
            }
        };

        si_information.push_str(status);
        si_information.push('|');
        si_information.push_str(&entry.header.record_number.to_string());
        si_information.push_str(mode_uid_gid);
        si_information.push_str(&filename_attribute.logical_size.to_string());
        si_information.push('|');

        fn_information.push_str(&si_information);
        fn_information.push_str(&filename_attribute.accessed.timestamp().to_string());
        fn_information.push('|');
        fn_information.push_str(&filename_attribute.mft_modified.timestamp().to_string());
        fn_information.push('|');
        fn_information.push_str(&filename_attribute.modified.timestamp().to_string());
        fn_information.push('|');
        fn_information.push_str(&filename_attribute.created.timestamp().to_string());
        fn_information.push('\n');

        let standard_info = mft.find_standard_information(&entry);
        si_information.push_str(&standard_info.accessed.timestamp().to_string());
        si_information.push('|');
        si_information.push_str(&standard_info.mft_modified.timestamp().to_string());
        si_information.push('|');
        si_information.push_str(&standard_info.modified.timestamp().to_string());
        si_information.push('|');
        si_information.push_str(&standard_info.created.timestamp().to_string());
        fn_information.push('\n');

        let parent = if filename_attribute.parent.entry == entry.header.record_number {
            log::warn!("this entry has no parent");
            None
        } else {
            Some(filename_attribute.parent)
        };

        Some(Bodyfile1Line {
            name: filename_attribute.name,
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
    pub fn format_fn(&self, full_name: &str) -> String {
        format!("0|{} ($FILE_NAME){}",
            full_name,
            self.fn_information
        )
    }

    fn find_attribute_list(entry: &mft::MftEntry) -> Option<AttributeListAttr> {
        entry.iter_attributes_matching(Some(vec!(MftAttributeType::AttributeList)))
             .find_map(Result::ok).and_then(|r| {
                match r.data {
                    MftAttributeContent::AttrX20(a) => Some(a),
                    _ => {
                        log::warn!("$MFT entry {} has AttributeList without entries", entry.header.record_number);
                        None
                    }
                }
            })
    }
}

struct Mft2BodyfileApplication {
    mft_file: PathBuf,
}

impl Mft2BodyfileApplication {
    pub fn new() -> Self {
        Self {
            mft_file: PathBuf::new(),
        }
    }

    fn parse_options(&mut self) -> Result<()> {
        let mut filename = String::new();
        {
            let mut ap = ArgumentParser::new();
            ap.set_description("parses an $MFT file to bodyfile (stdout)");
            ap.refer(&mut filename).add_argument("mft_file", Store, "path to $MFT").required();
            ap.parse_args_or_exit();
        }
    
        let fp = PathBuf::from(&filename);
        if ! (fp.exists() && fp.is_file()) {
            return Err(anyhow::Error::msg(format!("File {} does not exist", &filename)));
        } else {
            self.mft_file = fp;
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        self.parse_options()?;
        
        let mut parser = MftParser::from_path(&self.mft_file).unwrap();
        
        let pp = PreprocessedMft::new();
        for mft_entry in parser.iter_entries().filter_map(|e| if let Ok(m)=e {Some(m)} else {panic!("")}) {
            let reference = MftReference::new(mft_entry.header.record_number, mft_entry.header.sequence);
            let entry = PreprocessedMftEntry::new(&pp, mft_entry);
            pp.borrow_mut().insert(reference, entry);
        }
        let hundred_percent = pp.borrow().len();

        eprintln!("found {} entries in $MFT", hundred_percent);

        pp.borrow().update_bf_lines();
        pp.borrow().print_entries();
        Ok(())
    }
}

fn main() -> Result<()> {
    let _ = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stderr,
        ColorChoice::Auto);
    let mut app = Mft2BodyfileApplication::new();
    app.run()
}