pub mod active;
use active::{Active, Number};
use std::fmt::{Debug, Display};

fn display<T: Display>(x: &T) {
    println!("{}", x);
}

fn debug<T: Debug>(x: &T) {
    println!("[update/debug] {:?}", x);
}

fn main() {
    active::init_world();
    let mut x = Number::transformed(21.0, |x| x * 1.5);
    x.listen(display);
    x.update(1.0);
    x.listen(debug);
    x.update(2.0);
    x.unlisten(display);
    x.update(3.0);
    x.unlisten(debug);
    x.listen(display);
    x.update(4.0);
}