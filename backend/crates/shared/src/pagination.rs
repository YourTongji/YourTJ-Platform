//! Cursor pagination envelope. The platform uses opaque cursors (not page/offset)
//! on public list endpoints; admin endpoints may use page/limit instead.

use serde::{Deserialize, Serialize};

/// A page of results plus the cursor to fetch the next page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl<T> Page<T> {
    /// Build a page; `has_more` is derived from the presence of a next cursor.
    pub fn new(items: Vec<T>, next_cursor: Option<String>) -> Self {
        Self { has_more: next_cursor.is_some(), items, next_cursor }
    }

    /// A terminal page with no further results.
    pub fn last(items: Vec<T>) -> Self {
        Self { items, next_cursor: None, has_more: false }
    }
}
