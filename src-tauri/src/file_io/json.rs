use crate::db::types::JsonImportOptions;

pub fn build_read_expr(opts: &JsonImportOptions) -> String {
    let path = opts.file_path.replace('\'', "''");
    format!("read_json('{}')", path)
}
