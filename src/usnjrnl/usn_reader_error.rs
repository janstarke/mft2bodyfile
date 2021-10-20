use std::fmt;

pub enum UsnReaderError {
    IO(std::io::Error),
    SyntaxError(String),
    FailedToReadWindowsTime([u8;8]),
    NoMoreData,
  }
  
  impl From<std::io::Error> for UsnReaderError {
    fn from(err: std::io::Error) -> Self {
      Self::IO(err)
    }
  }
  
  impl fmt::Display for UsnReaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      match self {
        Self::IO(io_error) => write!(f, "IO Error: {}", io_error),
        Self::FailedToReadWindowsTime(data) => write!(f, "failed to read windows time: {:?}", data),
        Self::SyntaxError(err) => write!(f, "Syntax Error: {}", err),
        Self::NoMoreData => write!(f, "no more data"),
      }
    }
  }