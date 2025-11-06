//! Utility transform to add new statements before or after the specified
//! statement.
//!
//! `StatementInjectorStore` contains a `FxHashMap<StatementId,
//! Vec<AdjacentStatement>>`. It is stored on `TransformCtx`.
//!
//! `StatementInjector` transform inserts new statements before or after a
//! statement which is determined by the ID of the statement.
//!
//! Other transforms can add statements to the store with following methods:
//!
//! ```rs
//! self.ctx.statement_injector.insert_before(statement_id, statement);
//! self.ctx.statement_injector.insert_after(statement_id, statement);
//! self.ctx.statement_injector.insert_many_after(statement_id, statements);
//! ```

use std::{cell::RefCell, collections::hash_map::Entry};

use rustc_hash::FxHashMap;
use swc_common::Spanned;
use swc_ecma_ast::*;

/// Unique identifier for a statement based on its span.
///
/// In SWC, we use the span's low/high byte positions as a unique identifier
/// for statements, which serves the same purpose as OXC's `Address`.
pub type StatementId = (u32, u32);

/// Get a unique identifier for a statement.
pub trait GetStatementId {
    fn statement_id(&self) -> StatementId;
}

impl GetStatementId for Stmt {
    fn statement_id(&self) -> StatementId {
        let span = self.span();
        (span.lo.0, span.hi.0)
    }
}

#[derive(Debug)]
enum Direction {
    Before,
    After,
}

#[derive(Debug)]
struct AdjacentStatement {
    stmt: Stmt,
    direction: Direction,
}

/// Store for statements to be added to the statements.
pub struct StatementInjectorStore {
    insertions: RefCell<FxHashMap<StatementId, Vec<AdjacentStatement>>>,
}

// Public methods
impl StatementInjectorStore {
    /// Create new `StatementInjectorStore`.
    pub fn new() -> Self {
        Self {
            insertions: RefCell::new(FxHashMap::default()),
        }
    }
}

impl Default for StatementInjectorStore {
    fn default() -> Self {
        Self::new()
    }
}

// Insertion methods.
//
// Each of these functions is split into 2 parts:
//
// 1. Public outer function which is generic over any `GetStatementId`.
// 2. Private inner function which is non-generic and takes `StatementId`.
//
// Outer functions are marked `#[inline]`, as `GetStatementId::statement_id` is
// generally only 1 or 2 instructions. The non-trivial inner functions are not
// marked `#[inline]` - compiler can decide whether to inline or not.
impl StatementInjectorStore {
    /// Add a statement to be inserted immediately before the target statement.
    #[inline]
    pub fn insert_before<S: GetStatementId>(&self, target: &S, stmt: Stmt) {
        self.insert_before_id(target.statement_id(), stmt);
    }

    fn insert_before_id(&self, target: StatementId, stmt: Stmt) {
        let mut insertions = self.insertions.borrow_mut();
        let adjacent_stmts = insertions.entry(target).or_default();
        let index = adjacent_stmts
            .iter()
            .position(|s| matches!(s.direction, Direction::After))
            .unwrap_or(adjacent_stmts.len());
        adjacent_stmts.insert(
            index,
            AdjacentStatement {
                stmt,
                direction: Direction::Before,
            },
        );
    }

    /// Add a statement to be inserted immediately after the target statement.
    #[inline]
    pub fn insert_after<S: GetStatementId>(&self, target: &S, stmt: Stmt) {
        self.insert_after_id(target.statement_id(), stmt);
    }

    fn insert_after_id(&self, target: StatementId, stmt: Stmt) {
        let mut insertions = self.insertions.borrow_mut();
        let adjacent_stmts = insertions.entry(target).or_default();
        adjacent_stmts.push(AdjacentStatement {
            stmt,
            direction: Direction::After,
        });
    }

    /// Add multiple statements to be inserted immediately before the target
    /// statement.
    #[inline]
    pub fn insert_many_before<S, I>(&self, target: &S, stmts: I)
    where
        S: GetStatementId,
        I: IntoIterator<Item = Stmt>,
    {
        self.insert_many_before_id(target.statement_id(), stmts);
    }

    fn insert_many_before_id<I>(&self, target: StatementId, stmts: I)
    where
        I: IntoIterator<Item = Stmt>,
    {
        let mut insertions = self.insertions.borrow_mut();
        let adjacent_stmts = insertions.entry(target).or_default();
        adjacent_stmts.splice(
            0..0,
            stmts.into_iter().map(|stmt| AdjacentStatement {
                stmt,
                direction: Direction::Before,
            }),
        );
    }

    /// Add multiple statements to be inserted immediately after the target
    /// statement.
    #[inline]
    pub fn insert_many_after<S, I>(&self, target: &S, stmts: I)
    where
        S: GetStatementId,
        I: IntoIterator<Item = Stmt>,
    {
        self.insert_many_after_id(target.statement_id(), stmts);
    }

    fn insert_many_after_id<I>(&self, target: StatementId, stmts: I)
    where
        I: IntoIterator<Item = Stmt>,
    {
        let mut insertions = self.insertions.borrow_mut();
        let adjacent_stmts = insertions.entry(target).or_default();
        adjacent_stmts.extend(stmts.into_iter().map(|stmt| AdjacentStatement {
            stmt,
            direction: Direction::After,
        }));
    }

    /// Move insertions from one [`StatementId`] to another.
    ///
    /// Use this if you convert one statement to another, and other code may
    /// have attached insertions to the original statement.
    #[inline]
    pub fn move_insertions<S1: GetStatementId, S2: GetStatementId>(
        &self,
        old_target: &S1,
        new_target: &S2,
    ) {
        self.move_insertions_id(old_target.statement_id(), new_target.statement_id());
    }

    fn move_insertions_id(&self, old_id: StatementId, new_id: StatementId) {
        let mut insertions = self.insertions.borrow_mut();
        let Some(mut adjacent_stmts) = insertions.remove(&old_id) else {
            return;
        };

        match insertions.entry(new_id) {
            Entry::Occupied(entry) => {
                entry.into_mut().append(&mut adjacent_stmts);
            }
            Entry::Vacant(entry) => {
                entry.insert(adjacent_stmts);
            }
        }
    }
}

// Internal methods
impl StatementInjectorStore {
    /// Insert statements immediately before / after the target statement.
    ///
    /// This method should be called when visiting a `Vec<Stmt>` to inject
    /// any pending statements.
    pub fn insert_into_statements(&self, statements: &mut Vec<Stmt>) {
        let mut insertions = self.insertions.borrow_mut();
        if insertions.is_empty() {
            return;
        }

        let new_statement_count = statements
            .iter()
            .filter_map(|s| insertions.get(&s.statement_id()).map(Vec::len))
            .sum::<usize>();
        if new_statement_count == 0 {
            return;
        }

        let mut new_statements = Vec::with_capacity(statements.len() + new_statement_count);

        for stmt in statements.drain(..) {
            match insertions.remove(&stmt.statement_id()) {
                Some(mut adjacent_stmts) => {
                    let first_after_stmt_index = adjacent_stmts
                        .iter()
                        .position(|s| matches!(s.direction, Direction::After))
                        .unwrap_or(adjacent_stmts.len());
                    if first_after_stmt_index != 0 {
                        let right = adjacent_stmts.split_off(first_after_stmt_index);
                        new_statements.extend(adjacent_stmts.into_iter().map(|s| s.stmt));
                        new_statements.push(stmt);
                        new_statements.extend(right.into_iter().map(|s| s.stmt));
                    } else {
                        new_statements.push(stmt);
                        new_statements.extend(adjacent_stmts.into_iter().map(|s| s.stmt));
                    }
                }
                None => {
                    new_statements.push(stmt);
                }
            }
        }

        *statements = new_statements;
    }

    /// Assertion for checking if no remaining insertions are left.
    ///
    /// `#[inline(always)]` because this is a no-op in release mode.
    #[expect(clippy::inline_always)]
    #[inline(always)]
    pub fn assert_no_insertions_remaining(&self) {
        debug_assert!(self.insertions.borrow().is_empty());
    }
}

#[cfg(test)]
mod tests {
    use swc_ecma_ast::*;

    use super::*;

    fn dummy_stmt(id: u32) -> Stmt {
        Stmt::Empty(EmptyStmt {
            span: swc_common::Span::new(swc_common::BytePos(id), swc_common::BytePos(id + 1)),
        })
    }

    #[test]
    fn test_insert_before() {
        let store = StatementInjectorStore::new();
        let target = dummy_stmt(100);
        let to_insert = dummy_stmt(200);

        store.insert_before(&target, to_insert);

        let mut statements = vec![target];
        store.insert_into_statements(&mut statements);

        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].statement_id(), (200, 201));
        assert_eq!(statements[1].statement_id(), (100, 101));
    }

    #[test]
    fn test_insert_after() {
        let store = StatementInjectorStore::new();
        let target = dummy_stmt(100);
        let to_insert = dummy_stmt(200);

        store.insert_after(&target, to_insert);

        let mut statements = vec![target];
        store.insert_into_statements(&mut statements);

        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].statement_id(), (100, 101));
        assert_eq!(statements[1].statement_id(), (200, 201));
    }

    #[test]
    fn test_insert_many_before() {
        let store = StatementInjectorStore::new();
        let target = dummy_stmt(100);
        let to_insert = vec![dummy_stmt(200), dummy_stmt(201)];

        store.insert_many_before(&target, to_insert);

        let mut statements = vec![target];
        store.insert_into_statements(&mut statements);

        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0].statement_id(), (200, 201));
        assert_eq!(statements[1].statement_id(), (201, 202));
        assert_eq!(statements[2].statement_id(), (100, 101));
    }

    #[test]
    fn test_insert_many_after() {
        let store = StatementInjectorStore::new();
        let target = dummy_stmt(100);
        let to_insert = vec![dummy_stmt(200), dummy_stmt(201)];

        store.insert_many_after(&target, to_insert);

        let mut statements = vec![target];
        store.insert_into_statements(&mut statements);

        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0].statement_id(), (100, 101));
        assert_eq!(statements[1].statement_id(), (200, 201));
        assert_eq!(statements[2].statement_id(), (201, 202));
    }

    #[test]
    fn test_insert_before_and_after() {
        let store = StatementInjectorStore::new();
        let target = dummy_stmt(100);
        let before = dummy_stmt(200);
        let after = dummy_stmt(201);

        store.insert_before(&target, before);
        store.insert_after(&target, after);

        let mut statements = vec![target];
        store.insert_into_statements(&mut statements);

        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0].statement_id(), (200, 201));
        assert_eq!(statements[1].statement_id(), (100, 101));
        assert_eq!(statements[2].statement_id(), (201, 202));
    }

    #[test]
    fn test_move_insertions() {
        let store = StatementInjectorStore::new();
        let old_target = dummy_stmt(100);
        let new_target = dummy_stmt(300);
        let to_insert = dummy_stmt(200);

        store.insert_after(&old_target, to_insert);
        store.move_insertions(&old_target, &new_target);

        let mut statements = vec![new_target];
        store.insert_into_statements(&mut statements);

        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].statement_id(), (300, 301));
        assert_eq!(statements[1].statement_id(), (200, 201));
    }
}
