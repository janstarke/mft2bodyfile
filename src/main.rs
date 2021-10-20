mod intern;
mod usnjrnl;

use usnjrnl::*;
use intern::*;
use mft::MftParser;
use std::path::PathBuf;
use argparse::{ArgumentParser, Store};
use anyhow::Result;
use simplelog::{TermLogger, LevelFilter, Config, TerminalMode, ColorChoice};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;

struct Mft2BodyfileApplication {
    mft_file: PathBuf,
    usnjrnl: Option<PathBuf>
}

impl Mft2BodyfileApplication {
    pub fn new() -> Self {
        Self {
            mft_file: PathBuf::new(),
            usnjrnl: None,
        }
    }

    fn parse_options(&mut self) -> Result<()> {
        let mut filename = String::new();
        let mut usnjrnl_file = String::new();
        {
            let mut ap = ArgumentParser::new();
            ap.set_description("parses an $MFT file to bodyfile (stdout)");
            ap.refer(&mut filename).add_argument("mft_file", Store, "path to $MFT").required();
            ap.refer(&mut usnjrnl_file).add_option(&["-J", "--journal"], Store, "path to $UsnJrnl $J file (optional)");
            ap.parse_args_or_exit();
        }
    
        let fp = PathBuf::from(&filename);
        if ! (fp.exists() && fp.is_file()) {
            return Err(anyhow::Error::msg(format!("File {} does not exist", &filename)));
        } else {
            self.mft_file = fp;
        }

        if ! usnjrnl_file.is_empty() {
            let fp = PathBuf::from(&usnjrnl_file);
            if ! (fp.exists() && fp.is_file()) {
                return Err(anyhow::Error::msg(format!("File {} does not exist", &filename)));
            } else {
                self.usnjrnl = Some(fp);
            }
        }
        Ok(())
    }

    fn new_progress_bar(&self, message: &'static str, count:u64) -> ProgressBar {
        let bar = ProgressBar::new(count).with_message(message);
        bar.set_style(ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7}({percent}%) {msg}")
            .progress_chars("##-"));
        bar.set_draw_delta(1000);
        bar
    }

    pub fn run(&mut self) -> Result<()> {
        self.parse_options()?;

        /* read $UsnJrnl */
        let usnjrnl_path = self.usnjrnl.clone();
        let usnjrnl_thread = std::thread::spawn(|| {
            match usnjrnl_path {
                Some(jrnl_path) => UsnJrnl::from(UsnJrnlReader::from(&jrnl_path).unwrap()),
                None => UsnJrnl::new()
            }
        });
        
        let mut pp = PreprocessedMft::new();
        let mut parser = MftParser::from_path(&self.mft_file).unwrap();
        let bar = self.new_progress_bar("parsing $MFT entries", parser.get_entry_count());

        for mft_entry in parser.iter_entries().filter_map(Result::ok) {
            bar.inc(1);
            
            if (12..24).contains(&mft_entry.header.record_number) {
                //
                // ignore contents of $MFT extension entries
                //
                continue;
            } else if mft_entry.header.used_entry_size == 0 {  
                //
                // ignore unallocated entries without content
                //
                if mft_entry.is_allocated() {
                    log::info!("found allocated entry with zero entry size: {}", mft_entry.header.record_number);
                }
            } else {
                //
                // handle all other entries
                //
                pp.add_entry(mft_entry);
            }
        }
        bar.finish();
        let usnjrnl = usnjrnl_thread.join().unwrap();
        if usnjrnl.len() > 0 {
            let bar = self.new_progress_bar("merging $UsnJrnl entries", usnjrnl.len() as u64);
            for (reference, records) in usnjrnl.into_iter() {
                pp.add_usnjrnl_records(reference, records);
                bar.inc(1);
            }
            bar.finish();
        }

        let bar = self.new_progress_bar("exporting bodyfile lines", 2*pp.entries_count() as u64);
        let stdout = std::io::stdout();
        let mut stdout_lock = stdout.lock();
        for entry in pp.iter_entries() {
            stdout_lock.write_all(entry.as_bytes())?;
            bar.inc(1);
        }
        bar.finish();
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