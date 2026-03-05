pub struct SqliteRenderer;

impl SqliteRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqliteRenderer {
    fn default() -> Self {
        Self::new()
    }
}
