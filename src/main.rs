//I'd like the most pedantic warning level
#![warn(
    clippy::cargo,
    clippy::needless_borrow,
    clippy::pedantic,
    clippy::redundant_clone
)]
// But I don't care about these ones
#![allow(
    clippy::cast_precision_loss,     // There is no way to avoid this precision loss
    clippy::module_name_repetitions, // Sometimes clear naming calls for repetition
    clippy::multiple_crate_versions,  // There is no way to easily fix this without modifying our dependencies
    clippy::needless_pass_by_value // FIXME: Remove once the code is in a better state 
)]

#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate serde;

mod error;
mod request_handler;
mod router;
mod server;

use std::env;
use std::sync::Arc;

use crate::request_handler::HttpRequestHandler;

fn main() {
    let is_development_mode = env::args().any(|arg| arg == "--development");
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
