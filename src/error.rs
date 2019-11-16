use std::fmt::{self, Display, Formatter};
use std::str::Utf8Error;

use hyper::{Body, Response};

#[derive(Debug, Fail)]
pub enum RouterError {
    Hyper(hyper::error::Error),
    InternalJsonHandling(serde_json::Error),
    InvalidUtf8(Utf8Error),
}

impl Display for RouterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Hyper(e) => {
                write!(f, "RouterError, caused by internal hyper error: {}", e)?;
            }

            Self::InvalidUtf8(e) => {
                write!(
                    f,
                    "RouterError, caused by internal utf8 decode error: {}",
                    e
                )?;
            }

            Self::InternalJsonHandling(e) => {
                write!(f, "RouterError, caused by internal serde_json error: {}", e)?;
            }
        }
        Ok(())
    }
}

impl From<hyper::error::Error> for RouterError {
    fn from(e: hyper::error::Error) -> Self {
        Self::Hyper(e)
    }
}

impl From<Utf8Error> for RouterError {
    fn from(e: Utf8Error) -> Self {
        Self::InvalidUtf8(e)
    }
}

impl From<serde_json::Error> for RouterError {
    fn from(e: serde_json::Error) -> Self {
        Self::InternalJsonHandling(e)
    }
}

impl Into<Response<Body>> for RouterError {
    fn into(self) -> Response<Body> {
        Response::builder()
            .status(532)
            .body(Body::from(self.to_string()))
            .unwrap()
    }
}
