use std::{fmt, error, io};

#[derive(Debug)]
pub enum ReadError {
    InvalidState(String),
    IOError(io::Error),
    UnexpectedEofError,
}
impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ReadError::InvalidState(ref s) => write!(f, "Invalid State. ({})", s),
            ReadError::IOError(ref e) => write!(f, "{}", e),
            ReadError::UnexpectedEofError => write!(f, "Unexpected EOF.")
        }
    }
}
impl error::Error for ReadError {
    fn description(&self) -> &str {
        match *self {
            ReadError::InvalidState(_) => "Invalid State.",
            ReadError::IOError(_) => "IO Error.",
            ReadError::UnexpectedEofError => "UnexpectedEOF."
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            ReadError::InvalidState(_) => None,
            ReadError::IOError(ref e) => Some(e),
            ReadError::UnexpectedEofError => None,
        }
    }
}
impl From<io::Error> for ReadError {
    fn from(e: io::Error) -> Self {
        ReadError::IOError(e)
    }
}
#[derive(Debug)]
pub enum WriteError {
    InvalidState(String),
    IOError(io::Error),
}
impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WriteError::InvalidState(ref s) => write!(f, "Invalid State. ({})", s),
            WriteError::IOError(ref e) => write!(f, "{}", e),
        }
    }
}
impl error::Error for WriteError {
    fn description(&self) -> &str {
        match *self {
            WriteError::InvalidState(_) => "Invalid State.",
            WriteError::IOError(_) => "IO Error.",
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            WriteError::InvalidState(_) => None,
            WriteError::IOError(ref e) => Some(e),
        }
    }
}
impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        WriteError::IOError(e)
    }
}
#[derive(Debug)]
pub enum CompressionError {
    InvalidState(String),
    ReadError(ReadError),
    WriteError(WriteError)
}
impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CompressionError::InvalidState(ref s) => write!(f, "Invalid State. ({})", s),
            CompressionError::ReadError(ref e) => write!(f, "Read error ({})", e),
            CompressionError::WriteError(ref e) => write!(f, "Write error ({})", e)
        }
    }
}
impl error::Error for CompressionError {
    fn description(&self) -> &str {
        match *self {
            CompressionError::InvalidState(_) => "Invalid State.",
            CompressionError::ReadError(_) => "Read error.",
            CompressionError::WriteError(_) => "Write error."
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            CompressionError::InvalidState(_) => None,
            CompressionError::ReadError(ref e) => Some(e),
            CompressionError::WriteError(ref e) => Some(e)
        }
    }
}
impl From<ReadError> for CompressionError {
    fn from(e: ReadError) -> Self {
        CompressionError::ReadError(e)
    }
}
impl From<WriteError> for CompressionError {
    fn from(e: WriteError) -> Self {
        CompressionError::WriteError(e)
    }
}
