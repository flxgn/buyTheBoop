mod exchange;
mod strategy;
mod messaging;
mod tools;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate simple_error;

fn main() {
    tools::logging::setup();
}
