
mod exchange;
mod logging;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate simple_error;

fn main() {
    logging::setup();
}
