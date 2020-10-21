use crate::parser::{
    ast::{Expression, Field, Fn, Type},
    lex::Ident,
};
use std::collections::HashMap;

/// Infallible function allocator.
///
/// Panics instead of returning Optionals or Results, therefore a panic means a
/// bug somewhere in the compiler (likely in the frontend).
#[derive(Default)]
pub struct FnAlloc<'a> {
    fns: HashMap<String, &'a Fn<'a>>,
}

impl<'a> FnAlloc<'a> {
    /// Allocated a function from it's statement.
    /// Panics if a function of the same name is already allocated.
    pub fn alloc(&mut self, fn_: &'a Fn<'a>) {
        assert!(self.fns.insert(fn_.ident.to_string(), fn_).is_none())
    }

    /// Returns the function with the given name.
    /// Panics if it's not defined.
    pub fn get(&self, name: &str) -> &'a Fn<'a> {
        self.fns[name]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Space {
    /// Static memory.
    Static,
    /// Const memory (ROM).
    Const,
    /// Stack memory.
    Stack,
    /// Absolute memory space.
    Absolute,
}

#[derive(Debug, Clone)]
pub struct Symbol<'a> {
    /// Symbolic name.
    pub name: String,
    /// Offset in virtual memory.
    pub offset: u16,
    /// Size of the symbol itself.
    pub size: u16,
    /// The type of the symbol.
    pub type_: &'a Type<'a>,
    /// Virtual memory space.
    pub space: Space,
}

#[derive(Debug, Default, Clone)]
pub struct SymbolAlloc<'a> {
    absolute_symbols: Vec<Symbol<'a>>,
    const_symbols: Vec<Symbol<'a>>,
    static_symbols: Vec<Symbol<'a>>,
    stack_symbols: Vec<Symbol<'a>>,
    absolute_symbols_alloc: u16,
    const_symbols_alloc: u16,
    static_symbols_alloc: u16,
    stack_symbols_alloc: u16,
}

impl<'a> SymbolAlloc<'a> {
    /// Clear stack symbols
    pub fn clear_stack(&mut self) {
        self.stack_symbols.clear();
        self.stack_symbols_alloc = 0;
    }

    /// Allocate const address.
    pub fn alloc_const(&mut self, field: &'a Field<'a>, _expr: &Expression) {
        assert!(self.is_undefined(&field.ident));
        let size = Self::compute_all_symbols(
            &String::new(),
            self.const_symbols_alloc,
            field,
            Space::Const,
            &mut self.const_symbols,
        );
        self.const_symbols_alloc += size;
    }

    /// Allocate static address.
    pub fn alloc_static(&mut self, field: &'a Field<'a>) {
        assert!(self.is_undefined(&field.ident));
        let size = Self::compute_all_symbols(
            &String::new(),
            self.static_symbols_alloc,
            field,
            Space::Static,
            &mut self.static_symbols,
        );
        self.static_symbols_alloc += size;
    }

    /// Declares a symbol located at the given offset.
    /// Note that it is possible to overlap two symbols, as long as the language
    /// frontend allows it... (the IR doesn't really care about memory aliasing)
    pub fn alloc_absolute(&mut self, field: &'a Field<'a>, offset: u16) {
        assert!(self.is_undefined(&field.ident));
        Self::compute_all_symbols(
            &String::new(),
            offset,
            field,
            Space::Absolute,
            &mut self.absolute_symbols,
        );
    }

    /// Allocate stack address, associated to the given field.
    /// Returns the first allocated address.
    pub fn alloc_stack_field(&mut self, field: &'a Field<'a>) -> u16 {
        assert!(self.is_undefined(&field.ident));
        let size = Self::compute_all_symbols(
            &String::new(),
            self.stack_symbols_alloc,
            field,
            Space::Stack,
            &mut self.stack_symbols,
        );
        let alloc = self.stack_symbols_alloc;
        self.stack_symbols_alloc += size;
        alloc
    }

    /// Locates a symbol by name.
    /// Panics if the symbol is not defined.
    pub fn get(&self, name: &str) -> &Symbol {
        self.stack_symbols
            .iter()
            .chain(self.static_symbols.iter())
            .chain(self.const_symbols.iter())
            .chain(self.absolute_symbols.iter())
            .find(|s| &s.name == name)
            .expect("Undefined symbol")
    }

    fn is_undefined(&self, ident: &Ident) -> bool {
        !(Self::_is_undefined(ident, &self.absolute_symbols)
            || Self::_is_undefined(ident, &self.static_symbols)
            || Self::_is_undefined(ident, &self.const_symbols)
            || Self::_is_undefined(ident, &self.stack_symbols))
    }

    fn _is_undefined(ident: &Ident, symbols: &Vec<Symbol<'a>>) -> bool {
        symbols.iter().find(|s| &s.name == ident.as_str()).is_some()
    }

    // TODO optimize because I'm far too sleepy to do this now.
    //  No need to be calling size_of all over the place here.
    fn compute_all_symbols(
        prefix: &String,
        offset: u16,
        field: &'a Field<'a>,
        space: Space,
        symbols: &mut Vec<Symbol<'a>>,
    ) -> u16 {
        use Type::*;

        // append field identifier to the queried field.
        let name = if prefix.is_empty() {
            field.ident.to_string()
        } else {
            let mut prefix = prefix.clone();
            prefix.push_str(&format!("::{}", field.ident));
            prefix
        };

        // size of the entire field
        let size = super::utils::size_of(&field.type_);

        match &field.type_ {
            U8(_) | I8(_) | Array(_) | Pointer(_) | Fn(_) => {
                symbols.push(Symbol {
                    name,
                    offset,
                    size,
                    type_: &field.type_,
                    space,
                });
            }
            Struct(struct_) => {
                let mut offset = offset;
                for field in struct_.fields.iter() {
                    offset += Self::compute_all_symbols(&name, offset, field, space, symbols);
                }
            }
            Union(union) => {
                for field in union.fields.iter() {
                    Self::compute_all_symbols(&name, offset, field, space, symbols);
                }
            }
            _ => unreachable!(),
        }

        size
    }
}

/// Virtual register allocator.
#[derive(Default)]
pub struct RegisterAlloc {
    bitset: u64,
}

impl RegisterAlloc {
    /// Returns number of allocated registers.
    pub fn len(&self) -> u32 {
        self.bitset.count_ones()
    }

    /// Allocate register.
    pub fn alloc(&mut self) -> usize {
        let min = self.min();
        self.set(min, true);
        min
    }

    /// Free register being used.
    pub fn free(&mut self, index: usize) {
        assert!(self.get(index));
        self.set(index, false);
    }

    fn min(&self) -> usize {
        (0..64).find(|b| !self.get(*b)).unwrap()
    }

    fn get(&self, index: usize) -> bool {
        let bit = 1 << (index as u64);
        (self.bitset & bit) != 0
    }

    fn set(&mut self, index: usize, value: bool) -> bool {
        let bit = 1 << (index as u64);
        let old = (self.bitset | bit) != 0;
        if value {
            self.bitset |= bit;
        } else {
            self.bitset &= !bit;
        }
        old
    }
}

#[cfg(test)]
mod test {
    use super::RegisterAlloc;

    #[test]
    fn alloc() {
        let mut alloc = RegisterAlloc::default();

        alloc.alloc();
        alloc.alloc();
        alloc.alloc();
        alloc.alloc();
        assert_eq!(0b1111, alloc.bitset);
        alloc.set(1, false);
        assert_eq!(0b1101, alloc.bitset);
        alloc.alloc();
        assert_eq!(0b1111, alloc.bitset);
    }

    #[test]
    fn set() {
        let mut alloc = RegisterAlloc::default();
        alloc.set(0, true);
        alloc.set(2, true);
        alloc.set(4, true);
        alloc.set(6, true);
        assert!(alloc.get(0));
        assert!(!alloc.get(1));
        assert!(alloc.get(2));
        assert!(!alloc.get(3));
        assert!(alloc.get(4));
        assert!(!alloc.get(5));
        assert!(alloc.get(6));
        assert_eq!(0b1010101, alloc.bitset);
        alloc.set(2, false);
        alloc.set(4, false);
        alloc.set(6, false);
        assert_eq!(0b1, alloc.bitset);
        assert_eq!(1, alloc.min());
    }
}