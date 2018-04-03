extern crate bytes;
#[macro_use]
extern crate futures;
extern crate tokio;
extern crate tokio_io;
extern crate mio;
extern crate memchr;
extern crate libc;

mod threaded;

// GOAL #1: read stdin as a stream of futures
fn main() {
    threaded::main();
    println!("Hello, world!");
}
