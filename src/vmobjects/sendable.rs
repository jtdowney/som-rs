use std::fmt::Debug;

pub trait Sendable: Debug {
    fn send(&mut self, selector: String, arguments: Vec<Box<Sendable>>);
}
