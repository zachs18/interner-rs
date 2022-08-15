#![deny(unsafe_op_in_unsafe_fn)]

use std::rc::Rc;

use interner::{unsync::DataInterner, Interner, RcInterner};
use yoke::Yoke;

fn main() {

    let interner = Rc::new(DataInterner::new());

    let w = interner.yoked_add_str("Hello, world!");

    let x = interner.yoked_add_str("Lorem ipsum sit dolor amet.");

    let y = interner.find_str("Hello, world!").unwrap();

    let z = interner.yoked_find_str("Hello, world!Lorem").unwrap();

    dbg!(w.get(), w.get().as_ptr());
    dbg!(x.get(), x.get().as_ptr());
    dbg!(y, y.as_ptr());
    dbg!(z.get(), z.get().as_ptr());

    let borrowed: Yoke<&'static str, &str> = Yoke::attach_to_cart(x.get(), |string| {
        string
    });
    dbg!(borrowed.get(), borrowed.get().as_ptr());
}
