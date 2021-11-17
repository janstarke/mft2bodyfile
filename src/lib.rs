mod intern;

use usnjrnl::*;
pub use intern::*;
use mft::MftParser;
use std::path::PathBuf;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::fs::File;

pub enum BodyfileSink {
    Stdout,
    File(File)
}

pub struct Mft2BodyfileTask {
    mft_file: PathBuf,
    usnjrnl: Option<PathBuf>,
    usnjrnl_longflags: bool,
    with_progressbar: bool,
    output: BodyfileSink
}

impl Default for Mft2BodyfileTask {
    fn default() -> Self {
        Self {
            mft_file: PathBuf::new(),
            usnjrnl: None,
            usnjrnl_longflags: false,
            with_progressbar: false,
            output: BodyfileSink::Stdout,
        }
    }
}

impl Mft2BodyfileTask {
    pub fn with_mft_file(mut self, mft_file: PathBuf) -> Self {
        self.mft_file = mft_file;
        self
    }

    pub fn with_usnjrnl(mut self, usnjrnl: Option<PathBuf>) -> Self {
        self.usnjrnl = usnjrnl;
        self
    }

    pub fn with_usnjrnl_longflags(mut self, usnjrnl_longflags: bool) -> Self {
        self.usnjrnl_longflags = usnjrnl_longflags;
        self
    }

    pub fn with_progressbar(mut self, with_progressbar: bool) -> Self {
        self.with_progressbar = with_progressbar;
        self
    }

    pub fn with_output(mut self, output: BodyfileSink) -> Self {
        self.output = output;
        self
    }

    pub fn fill_preprocessed_mft<T>(mut parser: MftParser<T>, bar: Option<ProgressBar>) -> PreprocessedMft where T: std::io::Read + std::io::Seek{
        let mut pp = PreprocessedMft::default();
        for mft_entry in parser.iter_entries().filter_map(Result::ok) {
            if let Some(b) = bar.as_ref() {
                b.inc(1);
            }
            
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
        if let Some(b) = bar.as_ref() {
            b.finish();
        }
        pp
    }

    pub fn run(self) -> Result<()> {

        /* read $UsnJrnl in the background */
        let usnjrnl_path = self.usnjrnl.clone();
        let usnjrnl_thread = std::thread::spawn(|| {
            match usnjrnl_path {
                Some(jrnl_path) => UsnJrnl::from(UsnJrnlReader::from(&jrnl_path).unwrap()),
                None => UsnJrnl::default()
            }
        });
        
        let parser = MftParser::from_path(&self.mft_file).unwrap();
        let bar = self.new_progress_bar("parsing $MFT entries", parser.get_entry_count());
        let mut pp = Self::fill_preprocessed_mft(parser, Some(bar));

        let usnjrnl = usnjrnl_thread.join().unwrap();
        if ! usnjrnl.is_empty() {
            let bar = self.new_progress_bar("merging $UsnJrnl entries", usnjrnl.len() as u64);
            for (reference, records) in usnjrnl.into_iter() {
                pp.add_usnjrnl_records(reference, records);
                bar.inc(1);
            }
            bar.finish();
        }

        let bar = &self.new_progress_bar("exporting bodyfile lines", pp.bodyfile_lines_count() as u64);
        let stdout = std::io::stdout();
        let mut stdout_lock: Box<dyn Write> = match self.output {
            BodyfileSink::Stdout     => Box::new(stdout.lock()),
            BodyfileSink::File(file) => Box::new(file)
        };
        for entry in pp.iter_entries(self.usnjrnl_longflags) {
            stdout_lock.write_all(entry.as_bytes())?;
            stdout_lock.write_all("\n".as_bytes())?;
            bar.inc(1);
        }
        stdout_lock.flush()?;
        bar.finish();
        Ok(())
    }

    fn new_progress_bar(&self, message: &'static str, count:u64) -> ProgressBar {
        let bar = ProgressBar::new(count).with_message(message);
        bar.set_style(ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>9}/{len:9}({percent}%) {msg}")
            .progress_chars("##-"));
        bar.set_draw_delta(1000);
        bar
    }
}