mod intern;
use std::thread;
use usnjrnl::*;
pub use intern::*;
use mft::MftParser;
use std::path::PathBuf;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
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

    fn read_usnjrnl(usnjrnl_path: &Option<PathBuf>, bar: ProgressBar) -> UsnJrnl {
        match usnjrnl_path {
            Some(jrnl_path) => UsnJrnl::from(UsnJrnlReader::from(jrnl_path).unwrap(), bar),
            None => UsnJrnl::default()
        }
    }

    pub fn run(self) -> Result<()> {

        /* not to be mixed with MultiCar ;-) */
        let multi_bar = MultiProgress::new();

        let parser = MftParser::from_path(&self.mft_file).unwrap();
        let parser_bar = multi_bar.add(self.new_progress_bar("parsing $MFT entries", Some(parser.get_entry_count())));
        let pp_thread = thread::spawn(move||
            Self::fill_preprocessed_mft(parser, Some(parser_bar))
        );

        let usnjrnl_bar = multi_bar.add(self.new_progress_bar("parsing $UsnJrnl:$J entries", None));
        let usnjrnl_path = self.usnjrnl.clone();
        let usnjrnl_thread = thread::spawn(move||
            Self::read_usnjrnl(&usnjrnl_path, usnjrnl_bar)
        );
 
        let _ = multi_bar.join();
        let mut pp = pp_thread.join().unwrap();
        let usnjrnl = usnjrnl_thread.join().unwrap();

        if ! usnjrnl.is_empty() {
            let bar = self.new_progress_bar("merging $UsnJrnl entries", Some(usnjrnl.len() as u64));
            for (reference, records) in usnjrnl.into_iter() {
                pp.add_usnjrnl_records(reference, records);
                bar.inc(1);
            }
            bar.finish();
        }

        let bar = &self.new_progress_bar("exporting bodyfile lines", Some(pp.bodyfile_lines_count() as u64));
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

    fn new_progress_bar(&self, message: &'static str, count:Option<u64>) -> ProgressBar {
        let bar = match count {
            Some(count) => ProgressBar::new(count).with_message(message),
            None => ProgressBar::new_spinner().with_message(message)
        };
        let style = match count {
            Some(_count) => ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>9}/{len:9}({percent}%) {msg}")
            .progress_chars("##-"),
            None => ProgressStyle::default_spinner()
            .template("[{elapsed_precise}] {spinner:40} {pos:>9} {msg}")
            .tick_chars("|/-\\"),
        };
        bar.set_style(style);
        bar.set_draw_delta(1000);
        bar
    }
}