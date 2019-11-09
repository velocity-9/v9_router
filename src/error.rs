use std::fmt::{self, Display, Formatter};
use std::str::Utf8Error;

use hyper::{Body, Response, StatusCode};

#[derive(Debug, Fail)]
pub enum RouterError {
    Hyper(hyper::error::Error),
    InternalJsonHandling(serde_json::Error),
    InvalidUtf8(Utf8Error),
}

impl Display for RouterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            RouterError::Hyper(e) => {
                write!(f, "RouterError, caused by internal hyper error: {}", e)?;
            }

            RouterError::InvalidUtf8(e) => {
                write!(f, "RouterError, caused by internal utf8 decode error: {}", e)?;
            }

            RouterError::InternalJsonHandling(e) => {
                write!(f, "RouterError, caused by internal serde_json error: {}", e)?;
            }
        }
        Ok(())
    }
}

impl From<hyper::error::Error> for RouterError {
    fn from(e: hyper::error::Error) -> Self {
        RouterError::Hyper(e)
    }
}

impl From<Utf8Error> for RouterError {
    fn from(e: Utf8Error) -> Self {
        RouterError::InvalidUtf8(e)
    }
}

impl From<serde_json::Error> for RouterError {
    fn from(e: serde_json::Error) -> Self {
        RouterError::InternalJsonHandling(e)
    }
}

impl Into<Response<Body>> for RouterError {
    fn into(self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(self.to_string()))
            .unwrap()
    }
}
