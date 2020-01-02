use std::fmt::{self, Display, Formatter};
use std::str::Utf8Error;

use hyper::{Body, Response};

#[derive(Debug, Fail)]
pub enum RouterError {
    Hyper(hyper::error::Error),
    InternalJsonHandling(serde_json::Error),
    InvalidRequest(reqwest::Error),
    InvalidUtf8(Utf8Error),
    PathNotFound(String),
}

impl Display for RouterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Hyper(e) => {
                write!(f, "RouterError, caused by internal hyper error: {}", e)?;
            }

            Self::InternalJsonHandling(e) => {
                write!(f, "RouterError, caused by internal serde_json error: {}", e)?;
            }

            Self::InvalidRequest(e) => {
                write!(f, "RouterError, caused by an invalid reqwest response: {}", e)?;
            }

            Self::InvalidUtf8(e) => {
                write!(f, "RouterError, caused by internal utf8 decode error: {}", e)?;
            }

            Self::PathNotFound(p) => {
                write!(f, "RouterError, invalid path: {}", p)?;
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

impl From<reqwest::Error> for RouterError {
    fn from(e: reqwest::Error) -> Self {
        Self::InvalidRequest(e)
    }
}

impl From<serde_json::Error> for RouterError {
    fn from(e: serde_json::Error) -> Self {
        Self::InternalJsonHandling(e)
    }
}

impl From<Utf8Error> for RouterError {
    fn from(e: Utf8Error) -> Self {
        Self::InvalidUtf8(e)
    }
}

impl Into<Response<Body>> for RouterError {
    fn into(self) -> Response<Body> {
        let msg = self.to_string();
        let error_code = match self {
            Self::PathNotFound(_) => 404,
            _ => 532,
        };

        Response::builder()
            .status(error_code)
            .body(Body::from(msg))
            .unwrap()
    }
}
