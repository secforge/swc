//! RegExp Transformer
//!
//! This module supports various RegExp plugins to handle unsupported RegExp
//! literal features. When an unsupported feature is detected, these plugins
//! convert the RegExp literal into a `new RegExp()` constructor call to avoid
//! syntax errors.
//!
//! Note: You will need to include a polyfill for the `RegExp` constructor in
//! your code to have the correct runtime behavior.
//!
//! ### ES2015
//!
//! #### Sticky flag (`y`)
//! - @babel/plugin-transform-sticky-regex: <https://babeljs.io/docs/en/babel-plugin-transform-sticky-regex>
//!
//! #### Unicode flag (`u`)
//! - @babel/plugin-transform-unicode-regex: <https://babeljs.io/docs/en/babel-plugin-transform-unicode-regex>
//!
//! ### ES2018
//!
//! #### DotAll flag (`s`)
//! - @babel/plugin-transform-dotall-regex: <https://babeljs.io/docs/en/babel-plugin-transform-dotall-regex>
//! - Spec: ECMAScript 2018: <https://262.ecma-international.org/9.0/#sec-get-regexp.prototype.dotAll>
//!
//! #### Lookbehind assertions (`/(?<=x)/` and `/(?<!x)/`)
//! - Implementation: Same as esbuild's handling
//!
//! #### Named capture groups (`(?<name>x)`)
//! - @babel/plugin-transform-named-capturing-groups-regex: <https://babeljs.io/docs/en/babel-plugin-transform-named-capturing-groups-regex>
//!
//! #### Unicode property escapes (`\p{...}` and `\P{...}`)
//! - @babel/plugin-transform-unicode-property-regex: <https://babeljs.io/docs/en/babel-plugin-proposal-unicode-property-regex>
//!
//! ### ES2022
//!
//! #### Match indices flag (`d`)
//! - Implementation: Same as esbuild's handling
//!
//! ### ES2024
//!
//! #### Set notation + properties of strings (`v`)
//! - @babel/plugin-transform-unicode-sets-regex: <https://babeljs.io/docs/en/babel-plugin-proposal-unicode-sets-regex>
//! - TC39 Proposal: <https://github.com/tc39/proposal-regexp-set-notation>
//!
//! TODO(improve-on-babel): We could convert to plain `RegExp(...)` instead of
//! `new RegExp(...)`. TODO(improve-on-babel): When flags is empty, we could
//! output `RegExp("(?<=x)")` instead of `RegExp("(?<=x)", "")`. (actually these
//! would be improvements on ESBuild, not Babel)

use swc_atoms::Atom;
use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_regexp::LiteralParser;
use swc_ecma_regexp_ast::*;
use swc_ecma_regexp_visit::VisitWith;
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::compat::context::TransformCtx;

mod options;

pub use options::RegExpOptions;

/// RegExp transformer that converts unsupported RegExp features to constructor
/// calls.
pub struct RegExp<'ctx> {
    ctx: &'ctx TransformCtx,
    unsupported_flags: RegExpFlags,
    check_patterns: bool,
    pattern_checker: PatternChecker,
}

impl<'ctx> RegExp<'ctx> {
    /// Creates a new RegExp transformer with the given options.
    pub fn new(options: RegExpOptions, ctx: &'ctx TransformCtx) -> Self {
        // Build unsupported flags bitset
        let mut unsupported_flags = RegExpFlags::empty();
        if options.dot_all_flag {
            unsupported_flags.insert(RegExpFlags::DOT_ALL);
        }
        if options.sticky_flag {
            unsupported_flags.insert(RegExpFlags::STICKY);
        }
        if options.unicode_flag {
            unsupported_flags.insert(RegExpFlags::UNICODE);
        }
        if options.match_indices {
            unsupported_flags.insert(RegExpFlags::HAS_INDICES);
        }
        if options.set_notation {
            unsupported_flags.insert(RegExpFlags::UNICODE_SETS);
        }

        // Check if any pattern-based features need to be checked
        let check_patterns = options.look_behind_assertions
            || options.named_capture_groups
            || options.unicode_property_escapes;

        let pattern_checker = PatternChecker {
            look_behind_assertions: options.look_behind_assertions,
            named_capture_groups: options.named_capture_groups,
            unicode_property_escapes: options.unicode_property_escapes,
            found_unsupported: false,
        };

        Self {
            ctx,
            unsupported_flags,
            check_patterns,
            pattern_checker,
        }
    }

    /// Checks if the regex needs transformation and performs it if necessary.
    fn transform_regexp(&mut self, regex: &mut Regex) {
        let flags_str = regex.flags.as_ref();
        let flags = parse_flags(flags_str);

        // Check if any unsupported flags are present
        let has_unsupported_flags = flags.intersects(self.unsupported_flags);

        if !has_unsupported_flags && !self.check_patterns {
            // No transformation needed
            return;
        }

        // If we need to check patterns, parse the regex
        if !has_unsupported_flags && self.check_patterns {
            let pattern = match LiteralParser::new(
                regex.exp.as_ref(),
                Some(flags_str),
                swc_ecma_regexp::Options::default(),
            )
            .parse()
            {
                Ok(pattern) => pattern,
                Err(error) => {
                    self.ctx
                        .error(format!("Failed to parse regex: {:?}", error));
                    return;
                }
            };

            // Check if the pattern contains unsupported features
            let mut checker = self.pattern_checker;
            pattern.visit_with(&mut checker);

            if !checker.found_unsupported {
                // No unsupported patterns found
                return;
            }
        }

        // At this point, we need to transform the regex
        // The regex will be replaced by the visitor, so we don't need to do
        // anything here
    }
}

impl VisitMut for RegExp<'_> {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        // Visit children first
        expr.visit_mut_children_with(self);

        // Check if this is a regex literal that needs transformation
        if let Expr::Lit(Lit::Regex(regex)) = expr {
            let flags_str = regex.flags.as_ref();
            let flags = parse_flags(flags_str);

            // Check if any unsupported flags are present
            let has_unsupported_flags = flags.intersects(self.unsupported_flags);

            if !has_unsupported_flags && !self.check_patterns {
                return;
            }

            // If we need to check patterns, parse the regex
            if !has_unsupported_flags && self.check_patterns {
                let pattern = match LiteralParser::new(
                    regex.exp.as_ref(),
                    Some(flags_str),
                    swc_ecma_regexp::Options::default(),
                )
                .parse()
                {
                    Ok(pattern) => pattern,
                    Err(error) => {
                        self.ctx
                            .error(format!("Failed to parse regex: {:?}", error));
                        return;
                    }
                };

                // Check if the pattern contains unsupported features
                let mut checker = PatternChecker {
                    look_behind_assertions: self.pattern_checker.look_behind_assertions,
                    named_capture_groups: self.pattern_checker.named_capture_groups,
                    unicode_property_escapes: self.pattern_checker.unicode_property_escapes,
                    found_unsupported: false,
                };
                pattern.visit_with(&mut checker);

                if !checker.found_unsupported {
                    return;
                }
            }

            // Transform to new RegExp(...) constructor call
            let pattern_str = regex.exp.clone();
            let flags_str = regex.flags.clone();
            let span = regex.span;

            *expr = Expr::New(NewExpr {
                span,
                ctxt: Default::default(),
                callee: Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    ctxt: Default::default(),
                    sym: Atom::from("RegExp"),
                    optional: false,
                })),
                args: Some(vec![
                    ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Lit(Lit::Str(Str {
                            span: DUMMY_SP,
                            value: pattern_str.into(),
                            raw: None,
                        }))),
                    },
                    ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Lit(Lit::Str(Str {
                            span: DUMMY_SP,
                            value: flags_str.into(),
                            raw: None,
                        }))),
                    },
                ]),
                type_args: None,
            });
        }
    }
}

/// Helper to check if a pattern contains unsupported features
#[derive(Clone, Copy)]
struct PatternChecker {
    look_behind_assertions: bool,
    named_capture_groups: bool,
    unicode_property_escapes: bool,
    found_unsupported: bool,
}

impl swc_ecma_regexp_visit::Visit for PatternChecker {
    fn visit_look_around_assertion(&mut self, node: &LookAroundAssertion) {
        if self.look_behind_assertions {
            if matches!(
                node.kind,
                LookAroundAssertionKind::Lookbehind | LookAroundAssertionKind::NegativeLookbehind
            ) {
                self.found_unsupported = true;
            }
        }
        node.visit_children_with(self);
    }

    fn visit_capturing_group(&mut self, node: &CapturingGroup) {
        if self.named_capture_groups && node.name.is_some() {
            self.found_unsupported = true;
        }
        node.visit_children_with(self);
    }

    fn visit_unicode_property_escape(&mut self, _node: &UnicodePropertyEscape) {
        if self.unicode_property_escapes {
            self.found_unsupported = true;
        }
    }
}

/// Parse regex flags string into bitflags
fn parse_flags(flags: &str) -> RegExpFlags {
    let mut result = RegExpFlags::empty();
    for ch in flags.chars() {
        match ch {
            'g' => result.insert(RegExpFlags::GLOBAL),
            'i' => result.insert(RegExpFlags::IGNORE_CASE),
            'm' => result.insert(RegExpFlags::MULTILINE),
            's' => result.insert(RegExpFlags::DOT_ALL),
            'u' => result.insert(RegExpFlags::UNICODE),
            'y' => result.insert(RegExpFlags::STICKY),
            'd' => result.insert(RegExpFlags::HAS_INDICES),
            'v' => result.insert(RegExpFlags::UNICODE_SETS),
            _ => {}
        }
    }
    result
}

bitflags::bitflags! {
    /// RegExp flags as bitflags for efficient checking
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct RegExpFlags: u8 {
        const GLOBAL = 1 << 0;
        const IGNORE_CASE = 1 << 1;
        const MULTILINE = 1 << 2;
        const DOT_ALL = 1 << 3;
        const UNICODE = 1 << 4;
        const STICKY = 1 << 5;
        const HAS_INDICES = 1 << 6;
        const UNICODE_SETS = 1 << 7;
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use swc_common::DUMMY_SP;

    use super::*;
    use crate::Config;

    #[test]
    fn test_parse_flags() {
        let flags = parse_flags("gimsuy");
        assert!(flags.contains(RegExpFlags::GLOBAL));
        assert!(flags.contains(RegExpFlags::IGNORE_CASE));
        assert!(flags.contains(RegExpFlags::MULTILINE));
        assert!(flags.contains(RegExpFlags::DOT_ALL));
        assert!(flags.contains(RegExpFlags::UNICODE));
        assert!(flags.contains(RegExpFlags::STICKY));
    }

    #[test]
    fn test_no_transformation_when_not_needed() {
        let config = Config::default();
        let ctx = TransformCtx::new(Path::new("test.js"), &config);
        let options = RegExpOptions::default();
        let mut transform = RegExp::new(options, &ctx);

        let mut expr = Expr::Lit(Lit::Regex(Regex {
            span: DUMMY_SP,
            exp: Atom::from("test"),
            flags: Atom::from("g"),
        }));

        transform.visit_mut_expr(&mut expr);

        // Should still be a regex literal
        assert!(matches!(expr, Expr::Lit(Lit::Regex(_))));
    }

    #[test]
    fn test_transform_sticky_flag() {
        let config = Config::default();
        let ctx = TransformCtx::new(Path::new("test.js"), &config);
        let options = RegExpOptions {
            sticky_flag: true,
            ..Default::default()
        };
        let mut transform = RegExp::new(options, &ctx);

        let mut expr = Expr::Lit(Lit::Regex(Regex {
            span: DUMMY_SP,
            exp: Atom::from("test"),
            flags: Atom::from("y"),
        }));

        transform.visit_mut_expr(&mut expr);

        // Should be transformed to new RegExp(...)
        assert!(matches!(expr, Expr::New(_)));
    }
}
