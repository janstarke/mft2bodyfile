
mod intern;
use intern::*;

use mft::MftParser;
use mft::entry::EntryFlags;
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
            //
            // ignore contents of $MFT extension entries
            //
            if (12..24).contains(&mft_entry.header.record_number) {
                continue;
            } else
            
            //
            // ignore unallocated entries without content
            //
            if mft_entry.header.used_entry_size == 0 {
                let allocated_flag = mft_entry.header.flags & EntryFlags::ALLOCATED;
                if ! allocated_flag.is_empty() {
                    log::info!("found allocated entry with zero entry size: {}", mft_entry.header.record_number);
                }
            }
            
            //
            // handle all other entries
            //
            else {
                let reference = MftReference::new(mft_entry.header.record_number, mft_entry.header.sequence);

                if PreprocessedMft::is_base_entry(&mft_entry) {
                    let entry = PreprocessedMftEntry::new(&pp, mft_entry);
                    pp.borrow_mut().insert_base_entry(reference, entry);
                } else {
                    pp.borrow_mut().insert_nonbase_entry(reference, mft_entry);
                }
            }
        }
        //let hundred_percent = pp.borrow().len();

        //eprintln!("found {} entries in $MFT", hundred_percent);

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