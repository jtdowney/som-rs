use crate::vmobjects::{SObject, Sendable};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct SClass {
    pub superclass: Option<Rc<SObject>>,
    pub name: String,
    pub invokables: HashMap<String, Box<Sendable>>,
}
