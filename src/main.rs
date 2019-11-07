//I'd like the most pedantic warning level
#![warn(clippy::pedantic, clippy::needless_borrow)]
// But I don't care about these ones for now (most applicable since the code isn't fleshed out)
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::needless_pass_by_value)]

#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

extern crate serde;

use std::env;
use std::sync::Arc;

use crate::request_handler::HttpRequestHandler;

mod error;
mod request_handler;
mod router;
mod server;

fn main() {
    let is_development_mode = dbg!(env::args()).any(|arg| arg == "--development");
    if is_development_mode {
        println!("Starting in development mode");
    }

    let http_request_handler = HttpRequestHandler::new();

    server::start_server(
        is_development_mode,
        Arc::new(http_request_handler),
        request_handler::global_request_entrypoint,
    );
}
