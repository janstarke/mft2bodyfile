
mod intern;
use intern::*;

use mft::MftParser;
use std::path::PathBuf;
use winstructs::ntfs::mft_reference::MftReference;
use argparse::{ArgumentParser, Store};
use anyhow::Result;
use simplelog::{TermLogger, LevelFilter, Config, TerminalMode, ColorChoice};

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
        for mft_entry in parser.iter_entries().filter_map(Result::ok) {
            if mft_entry.header.used_entry_size == 0 {
                log::info!("found entry with zero entry size: {}", mft_entry.header.record_number);
            } else {
                let reference = MftReference::new(mft_entry.header.record_number, mft_entry.header.sequence);
                let entry = PreprocessedMftEntry::new(&pp, mft_entry);
                pp.borrow_mut().insert(reference, entry);
            }
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