use crate::db::types::CsvImportOptions;

/// Builds a DuckDB `read_csv(...)` expression from import options.
pub fn build_read_expr(opts: &CsvImportOptions) -> String {
    let path = opts.file_path.replace('\'', "''");
    let delim = match opts.delimiter.as_str() {
        "tab" => "\\t",
        "semicolon" => ";",
        _ => ",",
    };
    let encoding = if opts.encoding == "sjis" { "LATIN1" } else { "UTF-8" };
    format!(
        "read_csv('{}', header={}, delim='{}', encoding='{}')",
        path,
        if opts.has_header { "true" } else { "false" },
        delim,
        encoding
    )
}
