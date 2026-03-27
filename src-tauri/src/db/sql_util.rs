/// Escapes a SQL identifier (column/table/schema name) with double quotes.
pub fn ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Escapes a SQL string literal with single quotes.
pub fn literal(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Returns a fully qualified `"schema"."table"` identifier.
pub fn qualified(schema: &str, table: &str) -> String {
    format!("{}.{}", ident(schema), ident(table))
}
