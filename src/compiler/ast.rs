use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct Class {
    pub name: String,
    pub superclass: Option<String>,
    pub instance_methods: HashMap<String, Method>,
    pub instance_variables: Vec<String>,
    pub class_methods: HashMap<String, Method>,
    pub class_variables: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Assignment {
        variable: String,
        value: Box<Expression>,
    },
    BinaryMessage {
        message: String,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Block {
        parameters: Vec<String>,
        locals: Vec<String>,
        body: Vec<Expression>,
    },
    KeywordMessage {
        message: String,
        receiver: Box<Expression>,
        parameters: Vec<Expression>,
    },
    LiteralArray(Vec<Expression>),
    LiteralBoolean(bool),
    LiteralDouble(f64),
    LiteralInteger(i64),
    LiteralNil,
    LiteralString(String),
    LiteralSymbol(String),
    Return(Box<Expression>),
    UnaryMessage {
        message: String,
        receiver: Box<Expression>,
    },
    Variable(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Method {
    Primitive {
        name: String,
        parameters: Vec<String>,
    },
    Native {
        name: String,
        parameters: Vec<String>,
        locals: Vec<String>,
        body: Vec<Expression>,
    },
}
