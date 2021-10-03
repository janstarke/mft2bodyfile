mod intern;
use intern::*;

use mft::MftParser;
use mft::entry::EntryFlags;
use std::path::PathBuf;
use winstructs::ntfs::mft_reference::MftReference;
use argparse::{ArgumentParser, Store};
use anyhow::Result;
use simplelog::{TermLogger, LevelFilter, Config, TerminalMode, ColorChoice};
use indicatif::{ProgressBar, ProgressStyle};

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
        
        let mut pp = PreprocessedMft::new();
        let bar = ProgressBar::new(parser.get_entry_count()).with_message("parsing $MFT entries");
        bar.set_style(ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7}({percent}%) {msg}")
            .progress_chars("##-"));
        bar.set_draw_delta(1000);

        for mft_entry in parser.iter_entries().filter_map(Result::ok) {
            bar.inc(1);

            //
            // ignore contents of $MFT metadata entries
            //
            /*
            if mft_entry.header.record_number < 12 {
                continue;
            } else
            */

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
                if mft_entry.is_allocated() {
                    log::info!("found allocated entry with zero entry size: {}", mft_entry.header.record_number);
                }
            }
            
            //
            // handle all other entries
            //
            else {
                pp.add_entry(mft_entry);
            }
        }
        bar.finish();
        //let hundred_percent = pp.borrow().len();

        //eprintln!("found {} entries in $MFT", hundred_percent);

        //pp.borrow_mut().link_entries();
        //pp.borrow().update_bf_lines();
        pp.print_entries();
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