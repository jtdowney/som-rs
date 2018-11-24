use std::result;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Bytecode {
    Halt,
    Dup,
    PushLocal { index: u8, context: u8 },
    PushArgument { index: u8, context: u8 },
    PushField { index: u8 },
    PushBlock { index: u8 },
    PushConstant { index: u8 },
    PushGlobal { index: u8 },
    Pop,
    PopLocal { index: u8, context: u8 },
    PopArgument { index: u8, context: u8 },
    PopField { index: u8 },
    Send { index: u8 },
    SuperSend { index: u8 },
    ReturnLocal,
    ReturnNonLocal,
}

impl From<Bytecode> for Vec<u8> {
    fn from(source: Bytecode) -> Self {
        match source {
            Bytecode::Halt => vec![0],
            Bytecode::Dup => vec![1],
            Bytecode::PushLocal { index, context } => vec![2, index, context],
            Bytecode::PushArgument { index, context } => vec![3, index, context],
            Bytecode::PushField { index } => vec![4, index],
            Bytecode::PushBlock { index } => vec![5, index],
            Bytecode::PushConstant { index } => vec![6, index],
            Bytecode::PushGlobal { index } => vec![7, index],
            Bytecode::Pop => vec![8],
            Bytecode::PopLocal { index, context } => vec![9, index, context],
            Bytecode::PopArgument { index, context } => vec![10, index, context],
            Bytecode::PopField { index } => vec![11, index],
            Bytecode::Send { index } => vec![12, index],
            Bytecode::SuperSend { index } => vec![13, index],
            Bytecode::ReturnLocal => vec![14],
            Bytecode::ReturnNonLocal => vec![15],
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BytecodeIteratorError {
    UnknownBytecode(u8),
    InsufficientArguments,
}

type Result<T> = result::Result<T, BytecodeIteratorError>;

pub struct BytecodeIterator<I: Iterator<Item = u8>> {
    inner: I,
}

impl<I: Iterator<Item = u8>> BytecodeIterator<I> {
    pub fn new<T>(inner: T) -> BytecodeIterator<I>
    where
        T: IntoIterator<Item = u8, IntoIter = I>,
    {
        BytecodeIterator {
            inner: inner.into_iter(),
        }
    }

    fn read_bytecode(&mut self) -> Result<Option<Bytecode>> {
        let code = match self.inner.next() {
            Some(c) => c,
            None => return Ok(None),
        };

        let bytecode = match code {
            0 => Bytecode::Halt,
            1 => Bytecode::Dup,
            2 => Bytecode::PushLocal {
                index: self.read_argument()?,
                context: self.read_argument()?,
            },
            3 => Bytecode::PushArgument {
                index: self.read_argument()?,
                context: self.read_argument()?,
            },
            4 => Bytecode::PushField {
                index: self.read_argument()?,
            },
            5 => Bytecode::PushBlock {
                index: self.read_argument()?,
            },
            6 => Bytecode::PushConstant {
                index: self.read_argument()?,
            },
            7 => Bytecode::PushGlobal {
                index: self.read_argument()?,
            },
            8 => Bytecode::Pop,
            9 => Bytecode::PopLocal {
                index: self.read_argument()?,
                context: self.read_argument()?,
            },
            10 => Bytecode::PopArgument {
                index: self.read_argument()?,
                context: self.read_argument()?,
            },
            11 => Bytecode::PopField {
                index: self.read_argument()?,
            },
            12 => Bytecode::Send {
                index: self.read_argument()?,
            },
            13 => Bytecode::SuperSend {
                index: self.read_argument()?,
            },
            14 => Bytecode::ReturnLocal,
            15 => Bytecode::ReturnNonLocal,
            c => return Err(BytecodeIteratorError::UnknownBytecode(c)),
        };

        Ok(Some(bytecode))
    }

    fn read_argument(&mut self) -> Result<u8> {
        self.inner
            .next()
            .ok_or(BytecodeIteratorError::InsufficientArguments)
    }
}

impl<I: Iterator<Item = u8>> Iterator for BytecodeIterator<I> {
    type Item = Result<Bytecode>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_bytecode() {
            Ok(Some(c)) => Some(Ok(c)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytecode_iterator() {
        let bytecodes = BytecodeIterator::new(vec![2, 1, 2, 3, 3, 4, 12, 5])
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            vec![
                Bytecode::PushLocal {
                    index: 1,
                    context: 2
                },
                Bytecode::PushArgument {
                    index: 3,
                    context: 4
                },
                Bytecode::Send { index: 5 },
            ],
            bytecodes
        );
    }

    #[test]
    fn test_bytecode_iterator_unknown_bytecode() {
        let error = BytecodeIterator::new(vec![16])
            .collect::<Result<Vec<_>>>()
            .unwrap_err();
        assert_eq!(BytecodeIteratorError::UnknownBytecode(16), error);
    }

    #[test]
    fn test_bytecode_iterator_insufficient_arguments() {
        let error = BytecodeIterator::new(vec![3, 0])
            .collect::<Result<Vec<_>>>()
            .unwrap_err();
        assert_eq!(BytecodeIteratorError::InsufficientArguments, error);
    }
}
