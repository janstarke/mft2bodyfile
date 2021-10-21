mod intern;

pub use intern::*;
use std::path::PathBuf;
use argparse::{ArgumentParser, Store, StoreTrue};
use anyhow::Result;
use simplelog::{TermLogger, LevelFilter, Config, TerminalMode, ColorChoice};
use libmft2bodyfile::Mft2BodyfileTask;

struct Mft2BodyfileApplication {
    mft_file: PathBuf,
    usnjrnl: Option<PathBuf>,
    usnjrnl_longflags: bool,
}

impl Mft2BodyfileApplication {
    pub fn new() -> Self {
        Self {
            mft_file: PathBuf::new(),
            usnjrnl: None,
            usnjrnl_longflags: false
        }
    }

    fn parse_options(&mut self) -> Result<()> {
        let mut filename = String::new();
        let mut usnjrnl_file = String::new();
        let mut usnjrnl_longflags = false;
        {
            let mut ap = ArgumentParser::new();
            ap.set_description("parses an $MFT file to bodyfile (stdout)");
            ap.refer(&mut filename).add_argument("mft_file", Store, "path to $MFT").required();
            ap.refer(&mut usnjrnl_file).add_option(&["-J", "--journal"], Store, "path to $UsnJrnl $J file (optional)");
            ap.refer(&mut usnjrnl_longflags).add_option(&["--journal-long-flags"], StoreTrue, "don't remove the USN_REASON_ prefix from the $UsnJrnl reason output");
            ap.parse_args_or_exit();
        }
        self.usnjrnl_longflags = usnjrnl_longflags;
    
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

    pub fn run(mut self) -> Result<()> {
        self.parse_options()?;
        let task = Mft2BodyfileTask::new()
            .with_mft_file(self.mft_file)
            .with_usnjrnl(self.usnjrnl)
            .with_usnjrnl_longflags(self.usnjrnl_longflags)
            .with_progressbar(true)
            .with_output(Box::new(std::io::stdout()));
        task.run()
    }
}

fn main() -> Result<()> {
    let _ = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stderr,
        ColorChoice::Auto);
    let app = Mft2BodyfileApplication::new();
    app.run()
}