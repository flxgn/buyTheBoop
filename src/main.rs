mod exchanges;
mod processors;
mod strategies;
mod messages;
mod tools;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate simple_error;

fn main() {
    tools::logging::setup();
}
