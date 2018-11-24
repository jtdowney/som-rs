use crate::vmobjects::{SClass, Sendable};
use std::rc::Rc;

#[derive(Debug)]
pub struct SObject {
    class: Rc<SClass>,
    fields: Vec<Box<Sendable>>,
}
