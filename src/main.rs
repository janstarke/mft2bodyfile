mod intern;

pub use intern::*;
use std::path::PathBuf;
use clap::{App, Arg};
use anyhow::Result;
use simplelog::{TermLogger, LevelFilter, Config, TerminalMode, ColorChoice};
use libmft2bodyfile::{Mft2BodyfileTask, BodyfileSink};
use std::fs::File;

struct Mft2BodyfileApplication {
    mft_file: PathBuf,
    usnjrnl: Option<PathBuf>,
    output: BodyfileSink,
    usnjrnl_longflags: bool,
}

impl Mft2BodyfileApplication {
    pub fn new() -> Self {
        Self {
            mft_file: PathBuf::new(),
            usnjrnl: None,
            output: BodyfileSink::Stdout,
            usnjrnl_longflags: false
        }
    }

    fn parse_options(&mut self) -> Result<()> {
        let usnjrnl_help;
        let mft2bodyfile_help;

        if cfg!(feature = "gzip") {
            usnjrnl_help = "path to $UsnJrnl:$J file (optional; file ending with .gz will be treated as being gzipped)";
            mft2bodyfile_help = "path to $MFT (file ending with .gz will be treated as being gzipped)";
        } else {
            usnjrnl_help = "path to $UsnJrnl:$J file (optional)";
            mft2bodyfile_help = "path to $MFT";
        }

        let app = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author(env!("CARGO_PKG_AUTHORS"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .arg(
                Arg::with_name("MFT_FILE")
                    .help(mft2bodyfile_help)
                    .required(true)
                    .multiple(false)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("journal")
                    .short("J").long("journal")
                    .help(usnjrnl_help)
                    .takes_value(true)
                    .number_of_values(1)
            )
            .arg(
                Arg::with_name("journal-long-flags")
                    .long("journal-long-flags")
                    .help("don't remove the USN_REASON_ prefix from the $UsnJrnl reason output")
            )
            .arg(
                Arg::with_name("output")
                .short("O").long("output")
                .help("name of destination file (or '-' to write to stdout)")
                .takes_value(true)
                .number_of_values(1)
            );
        let matches = app.get_matches();
        self.usnjrnl_longflags = matches.is_present("journal-long-flags");
        let filename = matches.value_of("MFT_FILE").expect("missing $MFT filename");

        let fp = PathBuf::from(&filename);
        if ! (fp.exists() && fp.is_file()) {
            return Err(anyhow::Error::msg(format!("File {} does not exist", &filename)));
        } else {
            self.mft_file = fp;
        }

        if let Some(usnjrnl_filename) = matches.value_of("journal") {
            let fp = PathBuf::from(&usnjrnl_filename);
            if ! (fp.exists() && fp.is_file()) {
                return Err(anyhow::Error::msg(format!("File {} does not exist", &usnjrnl_filename)));
            } else {
                self.usnjrnl = Some(fp);
            }
        }

        if let Some(output) = matches.value_of("output") {
            if output != "-" {
                self.output = BodyfileSink::File(File::create(&output)?);
            }
        }

        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        self.parse_options()?;
        let task = Mft2BodyfileTask::default()
            .with_mft_file(self.mft_file)
            .with_usnjrnl(self.usnjrnl)
            .with_usnjrnl_longflags(self.usnjrnl_longflags)
            .with_progressbar(true)
            .with_output(self.output);
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