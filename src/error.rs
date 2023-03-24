use std::{fmt, error, io};

#[derive(Debug)]
pub enum ReadError {
    InvalidState(String),
    IOError(io::Error),
    UnexpectedEofError,
    InvalidArgumentError(String)
}
impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ReadError::InvalidState(ref s) => write!(f, "Invalid State. ({})", s),
            ReadError::IOError(ref e) => write!(f, "{}", e),
            ReadError::UnexpectedEofError => write!(f, "Unexpected EOF."),
            ReadError::InvalidArgumentError(ref s) => write!(f, "InvalidArgumentError ({})", s)
        }
    }
}
impl error::Error for ReadError {
    fn description(&self) -> &str {
        match *self {
            ReadError::InvalidState(_) => "Invalid State.",
            ReadError::IOError(_) => "IO Error.",
            ReadError::UnexpectedEofError => "UnexpectedEOF.",
            ReadError::InvalidArgumentError(_) => "Invalid argument."
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            ReadError::InvalidState(_) => None,
            ReadError::IOError(ref e) => Some(e),
            ReadError::UnexpectedEofError => None,
            ReadError::InvalidArgumentError(_) => None
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
    WriteError(WriteError),
    LimitError(String)
}
impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CompressionError::InvalidState(ref s) => write!(f, "Invalid State. ({})", s),
            CompressionError::ReadError(ref e) => write!(f, "Read error ({})", e),
            CompressionError::WriteError(ref e) => write!(f, "Write error ({})", e),
            CompressionError::LimitError(ref s) => write!(f, "limit error. ({})", s)
        }
    }
}
impl error::Error for CompressionError {
    fn description(&self) -> &str {
        match *self {
            CompressionError::InvalidState(_) => "Invalid State.",
            CompressionError::ReadError(_) => "Read error.",
            CompressionError::WriteError(_) => "Write error.",
            CompressionError::LimitError(_) => "limit error.",
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            CompressionError::InvalidState(_) => None,
            CompressionError::ReadError(ref e) => Some(e),
            CompressionError::WriteError(ref e) => Some(e),
            CompressionError::LimitError(_) => None
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
#[derive(Debug)]
pub enum UnCompressionError {
    InvalidState(String),
    ReadError(ReadError),
    WriteError(WriteError),
    FormatError
}
impl fmt::Display for UnCompressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            UnCompressionError::InvalidState(ref s) => write!(f, "Invalid State. ({})", s),
            UnCompressionError::ReadError(ref e) => write!(f, "Read error ({})", e),
            UnCompressionError::WriteError(ref e) => write!(f, "Write error ({})", e),
            UnCompressionError::FormatError => write!(f, "The format of the input is invalid.")
        }
    }
}
impl error::Error for UnCompressionError {
    fn description(&self) -> &str {
        match *self {
            UnCompressionError::InvalidState(_) => "Invalid State.",
            UnCompressionError::ReadError(_) => "Read error.",
            UnCompressionError::WriteError(_) => "Write error.",
            UnCompressionError::FormatError => "The format of the input is invalid."
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            UnCompressionError::InvalidState(_) => None,
            UnCompressionError::ReadError(ref e) => Some(e),
            UnCompressionError::WriteError(ref e) => Some(e),
            UnCompressionError::FormatError => None
        }
    }
}
impl From<ReadError> for UnCompressionError {
    fn from(e: ReadError) -> Self {
        UnCompressionError::ReadError(e)
    }
}
impl From<WriteError> for UnCompressionError {
    fn from(e: WriteError) -> Self {
        UnCompressionError::WriteError(e)
    }
}
