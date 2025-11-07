//! ES2022: Class Static Block
//!
//! This plugin transforms class static blocks (`class C { static { foo } }`) to
//! an equivalent using private fields (`class C { static #_ = foo }`).
//!
//! > This plugin is included in `preset-env`, in ES2022
//!
//! ## Example
//!
//! Input:
//! ```js
//! class C {
//!   static {
//!     foo();
//!   }
//!   static {
//!     foo();
//!     bar();
//!   }
//! }
//! ```
//!
//! Output:
//! ```js
//! class C {
//!   static #_ = foo();
//!   static #_2 = (() => {
//!     foo();
//!     bar();
//!   })();
//! }
//! ```
//!
//! ## Implementation
//!
//! Implementation based on [@babel/plugin-transform-class-static-block](https://babel.dev/docs/babel-plugin-transform-class-static-block).
//!
//! ## References:
//! * Babel plugin implementation: <https://github.com/babel/babel/tree/v7.26.2/packages/babel-plugin-transform-class-static-block>
//! * Class static initialization blocks TC39 proposal: <https://github.com/tc39/proposal-class-static-block>

use itoa::Buffer as ItoaBuffer;
use swc_atoms::Atom;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast::*;
use swc_ecma_hooks::VisitMutHook;

use crate::compat::utils::ast_builder::wrap_statements_in_arrow_function_iife;

/// Transforms class static blocks to private field initializers.
pub struct ClassStaticBlock;

impl ClassStaticBlock {
    /// Create a new ClassStaticBlock transformer.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

impl VisitMutHook for ClassStaticBlock {
    fn enter_class(&mut self, class: &mut Class) {
        // Loop through class body elements and:
        // 1. Find if there are any `StaticBlock`s.
        // 2. Collate list of private keys matching `#_` or `#_[1-9]...`.
        //
        // Don't collate private keys list conditionally only if a static block is
        // found, as usually there will be no matching private keys, so those
        // checks are cheap and will not allocate.
        let mut has_static_block = false;
        let mut keys = Keys::default();

        for member in &class.body {
            match member {
                ClassMember::StaticBlock(_) => {
                    has_static_block = true;
                }
                ClassMember::Method(method) => {
                    if let PropName::Ident(ident) = &method.key {
                        keys.reserve(&ident.sym);
                    }
                }
                ClassMember::PrivateMethod(method) => {
                    keys.reserve(&method.key.name);
                }
                ClassMember::ClassProp(prop) => {
                    if let PropName::Ident(ident) = &prop.key {
                        keys.reserve(&ident.sym);
                    }
                }
                ClassMember::PrivateProp(prop) => {
                    keys.reserve(&prop.key.name);
                }
                ClassMember::AutoAccessor(accessor) => match &accessor.key {
                    Key::Private(private_name) => {
                        keys.reserve(&private_name.name);
                    }
                    Key::Public(PropName::Ident(ident)) => {
                        keys.reserve(&ident.sym);
                    }
                    Key::Public(_) => {}
                },
                ClassMember::Constructor(_)
                | ClassMember::TsIndexSignature(_)
                | ClassMember::Empty(_) => {}
            }
        }

        // Transform static blocks
        if !has_static_block {
            return;
        }

        for member in &mut class.body {
            if let ClassMember::StaticBlock(block) = member {
                *member = Self::convert_block_to_private_field(block, &mut keys);
            }
        }
    }
}

impl ClassStaticBlock {
    /// Convert static block to private field.
    /// `static { foo }` -> `static #_ = foo;`
    /// `static { foo; bar; }` -> `static #_ = (() => { foo; bar; })();`
    fn convert_block_to_private_field(block: &mut StaticBlock, keys: &mut Keys) -> ClassMember {
        let expr = Self::convert_block_to_expression(block);

        let key_name = keys.get_unique();
        let key = PrivateName {
            span: DUMMY_SP,
            name: key_name,
        };

        ClassMember::PrivateProp(PrivateProp {
            span: block.span,
            ctxt: SyntaxContext::empty(),
            key,
            value: Some(Box::new(expr)),
            type_ann: None,
            is_static: true,
            decorators: vec![],
            accessibility: None,
            is_optional: false,
            is_override: false,
            readonly: false,
            definite: false,
        })
    }

    /// Convert static block to expression which will be value of private field.
    /// `static { foo }` -> `foo`
    /// `static { foo; bar; }` -> `(() => { foo; bar; })()`
    fn convert_block_to_expression(block: &mut StaticBlock) -> Expr {
        // If block contains only a single `ExpressionStatement`, no need to wrap in an
        // IIFE. `static { foo }` -> `foo`
        let stmts = &mut block.body.stmts;
        if stmts.len() == 1 {
            if let Some(Stmt::Expr(stmt)) = stmts.first_mut() {
                // Take the expression from the statement
                let expr = std::mem::replace(
                    &mut stmt.expr,
                    Box::new(Expr::Invalid(Invalid { span: DUMMY_SP })),
                );
                return *expr;
            }
        }

        // Convert block to arrow function IIFE.
        // `static { foo; bar; }` -> `(() => { foo; bar; })()`
        let stmts = std::mem::take(&mut block.body.stmts);
        wrap_statements_in_arrow_function_iife(stmts, block.span)
    }
}

/// Store of private identifier keys matching `#_` or `#_[1-9]...`.
///
/// Most commonly there will be no existing keys matching this pattern
/// (why would you prefix a private key with `_`?).
/// It's also uncommon to have more than 1 static block in a class.
///
/// Therefore common case is only 1 static block, which will use key `#_`.
/// So store whether `#_` is in set as a separate `bool`, to make a fast path
/// this common case, which does not involve any allocations (`numbered` will
/// remain empty).
///
/// Use a `Vec` rather than a `HashMap`, because number of matching private keys
/// is usually small, and `Vec` is lower overhead in that case.
#[derive(Default)]
struct Keys {
    /// `true` if keys includes `#_`.
    underscore: bool,
    /// Keys matching `#_[1-9]...`. Stored without the `_` prefix.
    numbered: Vec<Atom>,
}

impl Keys {
    /// Add a key to set.
    ///
    /// Key will only be added to set if it's `_`, or starts with `_[1-9]`.
    fn reserve(&mut self, key: &Atom) {
        let key_str = key.as_str();
        let mut bytes = key_str.as_bytes().iter().copied();
        if bytes.next() != Some(b'_') {
            return;
        }

        match bytes.next() {
            None => {
                self.underscore = true;
            }
            Some(b'1'..=b'9') => {
                self.numbered.push(Atom::from(&key_str[1..]));
            }
            _ => {}
        }
    }

    /// Get a key which is not in the set.
    ///
    /// Returned key will be either `_`, or `_<integer>` starting with `_2`.
    #[inline]
    fn get_unique(&mut self) -> Atom {
        #[expect(clippy::if_not_else)]
        if !self.underscore {
            self.underscore = true;
            Atom::from("_")
        } else {
            self.get_unique_slow()
        }
    }

    /// `#[cold]` and `#[inline(never)]` as it should be very rare to need a key
    /// other than `#_`.
    #[cold]
    #[inline(never)]
    fn get_unique_slow(&mut self) -> Atom {
        // Source text length is limited to `u32::MAX` so impossible to have more than
        // `u32::MAX` private keys. So `u32` is sufficient here.
        let mut i = 2u32;
        let mut buffer = ItoaBuffer::new();
        let mut num_str;
        loop {
            num_str = buffer.format(i);
            let num_atom = Atom::from(num_str);
            if !self.numbered.contains(&num_atom) {
                break;
            }
            i += 1;
        }

        let key = Atom::from(format!("_{num_str}"));
        self.numbered.push(Atom::from(&key.as_str()[1..]));

        key
    }
}

#[cfg(test)]
mod test {
    use super::Keys;

    #[test]
    fn keys_no_reserved() {
        let mut keys = Keys::default();

        assert_eq!(keys.get_unique(), "_");
        assert_eq!(keys.get_unique(), "_2");
        assert_eq!(keys.get_unique(), "_3");
        assert_eq!(keys.get_unique(), "_4");
        assert_eq!(keys.get_unique(), "_5");
        assert_eq!(keys.get_unique(), "_6");
        assert_eq!(keys.get_unique(), "_7");
        assert_eq!(keys.get_unique(), "_8");
        assert_eq!(keys.get_unique(), "_9");
        assert_eq!(keys.get_unique(), "_10");
        assert_eq!(keys.get_unique(), "_11");
        assert_eq!(keys.get_unique(), "_12");
    }

    #[test]
    fn keys_no_relevant_reserved() {
        let mut keys = Keys::default();
        keys.reserve(&"a".into());
        keys.reserve(&"foo".into());
        keys.reserve(&"__".into());
        keys.reserve(&"_0".into());
        keys.reserve(&"_1".into());
        keys.reserve(&"_a".into());
        keys.reserve(&"_foo".into());
        keys.reserve(&"_2foo".into());

        assert_eq!(keys.get_unique(), "_");
        assert_eq!(keys.get_unique(), "_2");
        assert_eq!(keys.get_unique(), "_3");
    }

    #[test]
    fn keys_reserved_underscore() {
        let mut keys = Keys::default();
        keys.reserve(&"_".into());

        assert_eq!(keys.get_unique(), "_2");
        assert_eq!(keys.get_unique(), "_3");
        assert_eq!(keys.get_unique(), "_4");
    }

    #[test]
    fn keys_reserved_numbers() {
        let mut keys = Keys::default();
        keys.reserve(&"_2".into());
        keys.reserve(&"_4".into());
        keys.reserve(&"_11".into());

        assert_eq!(keys.get_unique(), "_");
        assert_eq!(keys.get_unique(), "_3");
        assert_eq!(keys.get_unique(), "_5");
        assert_eq!(keys.get_unique(), "_6");
        assert_eq!(keys.get_unique(), "_7");
        assert_eq!(keys.get_unique(), "_8");
        assert_eq!(keys.get_unique(), "_9");
        assert_eq!(keys.get_unique(), "_10");
        assert_eq!(keys.get_unique(), "_12");
    }

    #[test]
    fn keys_reserved_later_numbers() {
        let mut keys = Keys::default();
        keys.reserve(&"_5".into());
        keys.reserve(&"_4".into());
        keys.reserve(&"_12".into());
        keys.reserve(&"_13".into());

        assert_eq!(keys.get_unique(), "_");
        assert_eq!(keys.get_unique(), "_2");
        assert_eq!(keys.get_unique(), "_3");
        assert_eq!(keys.get_unique(), "_6");
        assert_eq!(keys.get_unique(), "_7");
        assert_eq!(keys.get_unique(), "_8");
        assert_eq!(keys.get_unique(), "_9");
        assert_eq!(keys.get_unique(), "_10");
        assert_eq!(keys.get_unique(), "_11");
        assert_eq!(keys.get_unique(), "_14");
    }

    #[test]
    fn keys_reserved_underscore_and_numbers() {
        let mut keys = Keys::default();
        keys.reserve(&"_2".into());
        keys.reserve(&"_4".into());
        keys.reserve(&"_".into());

        assert_eq!(keys.get_unique(), "_3");
        assert_eq!(keys.get_unique(), "_5");
        assert_eq!(keys.get_unique(), "_6");
    }

    #[test]
    fn keys_reserved_underscore_and_later_numbers() {
        let mut keys = Keys::default();
        keys.reserve(&"_5".into());
        keys.reserve(&"_4".into());
        keys.reserve(&"_".into());

        assert_eq!(keys.get_unique(), "_2");
        assert_eq!(keys.get_unique(), "_3");
        assert_eq!(keys.get_unique(), "_6");
    }
}
