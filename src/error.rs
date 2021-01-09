pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum ClipboardError {
    Timeout,
    DuplicatedValue,
    EmptyValue,
    Other(x11_clipboard::error::Error),
}

#[derive(Debug)]
pub enum Error {
    ClipboardError(ClipboardError),
    WebserverError(actix_web::Error),
    IO(std::io::Error),
    UTF8Error(std::string::FromUtf8Error),
}

impl From<actix_web::Error> for Error {
    fn from(e: actix_web::Error) -> Error {
        Error::WebserverError(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::IO(e)
    }
}

impl From<x11_clipboard::error::Error> for Error {
    fn from(e: x11_clipboard::error::Error) -> Error {
        Error::ClipboardError(ClipboardError::Other(e))
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Error {
        Error::UTF8Error(e)
    }
}
