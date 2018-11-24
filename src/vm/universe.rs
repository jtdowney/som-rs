use crate::vmobjects::SSymbol;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Universe {
    symbols: HashMap<String, Rc<SSymbol>>,
}

impl Universe {
    pub fn new() -> Universe {
        Universe {
            symbols: HashMap::new(),
        }
    }

    pub fn load_symbol(&mut self, text: &str) -> Rc<SSymbol> {
        if self.symbols.contains_key(text) {
            self.symbols[text].clone()
        } else {
            let symbol = Rc::new(SSymbol(text.into()));
            self.symbols.insert(text.into(), symbol.clone());
            symbol
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_symbol_creates_symbol() {
        let mut universe = Universe::new();
        let symbol = universe.load_symbol("test");
        assert_eq!(&SSymbol("test".into()), symbol.as_ref());
    }

    #[test]
    fn test_load_symbol_returns_same_symbol() {
        let mut universe = Universe::new();
        let symbol1 = universe.load_symbol("test");
        let symbol2 = universe.load_symbol("test");
        assert!(Rc::ptr_eq(&symbol1, &symbol2));
    }
}
