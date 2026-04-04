use tiny_data_ware_house_lib::db::types::{CsvImportOptions, JsonImportOptions};
use tiny_data_ware_house_lib::db::worker::DbWorker;

fn test_worker() -> DbWorker {
    DbWorker::new(":memory:", "")
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_query_basic() {
    let worker = test_worker();
    let result = worker.query("SELECT 42 AS answer".to_string()).await.unwrap();
    assert_eq!(result.columns, vec!["answer"]);
    assert_eq!(result.rows[0][0], serde_json::json!(42));
}

#[tokio::test]
async fn test_query_invalid_sql_returns_error() {
    let worker = test_worker();
    let result = worker.query("NOT VALID SQL".to_string()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_query_truncated_flag() {
    let worker = test_worker();
    worker.query("CREATE TABLE big AS SELECT range AS id FROM range(1001)".to_string()).await.unwrap();
    let result = worker.query("SELECT * FROM big".to_string()).await.unwrap();
    assert!(result.truncated);
    assert_eq!(result.rows.len(), 1000);
}

// ---------------------------------------------------------------------------
// ListTables
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_tables_initially_empty() {
    let worker = test_worker();
    let tables = worker.list_tables().await.unwrap();
    assert!(tables.is_empty());
}

#[tokio::test]
async fn test_list_tables_after_create() {
    let worker = test_worker();
    worker.query("CREATE TABLE foo (id INTEGER, name VARCHAR)".to_string()).await.unwrap();
    let tables = worker.list_tables().await.unwrap();
    assert!(tables.iter().any(|t| t.name == "foo"));
}

// ---------------------------------------------------------------------------
// ListDbtTables
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: dbt DB fixture not yet implemented"]
async fn test_list_dbt_tables() {
    let _worker = test_worker();
    todo!("dbt DB fixture が用意できたら実装する");
}

// ---------------------------------------------------------------------------
// PreviewDbtTable
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: dbt DB fixture not yet implemented"]
async fn test_preview_dbt_table() {
    let _worker = test_worker();
    todo!("dbt DB fixture が用意できたら実装する");
}

// ---------------------------------------------------------------------------
// GetSchema
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_schema() {
    let worker = test_worker();
    worker.query("CREATE TABLE bar (id INTEGER, label VARCHAR)".to_string()).await.unwrap();
    let schema = worker.get_schema("main".to_string(), "bar".to_string()).await.unwrap();
    let col_names: Vec<&str> = schema.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"id"));
    assert!(col_names.contains(&"label"));
}

// ---------------------------------------------------------------------------
// PreviewTable
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: not yet implemented"]
async fn test_preview_table() {
    let _worker = test_worker();
    todo!("テーブル作成後に preview_table を呼んで行数を検証する");
}

// ---------------------------------------------------------------------------
// PreviewCsv
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: CSV fixture not yet implemented"]
async fn test_preview_csv() {
    let _opts = CsvImportOptions {
        file_path: "/tmp/dummy.csv".to_string(),
        table_name: "dummy".to_string(),
        schema_name: "main".to_string(),
        has_header: true,
        delimiter: ",".to_string(),
        encoding: "utf8".to_string(),
        if_exists: "replace".to_string(),
    };
    todo!("CSV fixture ファイルを用意して preview_csv を検証する");
}

// ---------------------------------------------------------------------------
// ImportCsv
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: CSV fixture not yet implemented"]
async fn test_import_csv() {
    let _opts = CsvImportOptions {
        file_path: "/tmp/dummy.csv".to_string(),
        table_name: "dummy".to_string(),
        schema_name: "main".to_string(),
        has_header: true,
        delimiter: ",".to_string(),
        encoding: "utf8".to_string(),
        if_exists: "replace".to_string(),
    };
    todo!("CSV fixture ファイルを用意して import_csv を検証する");
}

// ---------------------------------------------------------------------------
// ListSchemas
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: not yet implemented"]
async fn test_list_schemas() {
    let _worker = test_worker();
    todo!("list_schemas が 'main' を含む結果を返すことを検証する");
}

// ---------------------------------------------------------------------------
// AttachDbt / DetachDbt
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: dbt DB fixture not yet implemented"]
async fn test_attach_and_detach_dbt() {
    let _worker = test_worker();
    todo!("dbt DB ファイルを用意して attach_dbt / detach_dbt を検証する");
}

// ---------------------------------------------------------------------------
// GetDbtSchema
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: dbt DB fixture not yet implemented"]
async fn test_get_dbt_schema() {
    let _worker = test_worker();
    todo!("dbt DB fixture が用意できたら実装する");
}

// ---------------------------------------------------------------------------
// DropDbtTable
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: dbt DB fixture not yet implemented"]
async fn test_drop_dbt_table() {
    let _worker = test_worker();
    todo!("dbt DB fixture が用意できたら実装する");
}

// ---------------------------------------------------------------------------
// DropDbtSchema
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: dbt DB fixture not yet implemented"]
async fn test_drop_dbt_schema() {
    let _worker = test_worker();
    todo!("dbt DB fixture が用意できたら実装する");
}

// ---------------------------------------------------------------------------
// GetTableMeta
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: not yet implemented"]
async fn test_get_table_meta() {
    let _worker = test_worker();
    todo!("テーブル作成後に get_table_meta を呼んで schema_name / table_name を検証する");
}

// ---------------------------------------------------------------------------
// GetDbtTableMeta
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: dbt DB fixture not yet implemented"]
async fn test_get_dbt_table_meta() {
    let _worker = test_worker();
    todo!("dbt DB fixture が用意できたら実装する");
}

// ---------------------------------------------------------------------------
// SetTableComment
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: not yet implemented"]
async fn test_set_table_comment() {
    let _worker = test_worker();
    todo!("テーブル作成後に set_table_comment を呼んで get_table_meta でコメントを検証する");
}

// ---------------------------------------------------------------------------
// SetColumnComment
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: not yet implemented"]
async fn test_set_column_comment() {
    let _worker = test_worker();
    todo!("テーブル作成後に set_column_comment を呼んで get_table_meta でコメントを検証する");
}

// ---------------------------------------------------------------------------
// ReimportCsv
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: CSV fixture not yet implemented"]
async fn test_reimport_csv() {
    let _worker = test_worker();
    todo!("import_csv 後に reimport_csv を呼んでテーブルが更新されることを検証する");
}

// ---------------------------------------------------------------------------
// PreviewJson
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: JSON fixture not yet implemented"]
async fn test_preview_json() {
    let _opts = JsonImportOptions {
        file_path: "/tmp/dummy.json".to_string(),
        table_name: "dummy".to_string(),
        schema_name: "main".to_string(),
        if_exists: "replace".to_string(),
    };
    todo!("JSON fixture ファイルを用意して preview_json を検証する");
}

// ---------------------------------------------------------------------------
// ImportJson
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: JSON fixture not yet implemented"]
async fn test_import_json() {
    let _opts = JsonImportOptions {
        file_path: "/tmp/dummy.json".to_string(),
        table_name: "dummy".to_string(),
        schema_name: "main".to_string(),
        if_exists: "replace".to_string(),
    };
    todo!("JSON fixture ファイルを用意して import_json を検証する");
}

// ---------------------------------------------------------------------------
// TouchTableTimestamp
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: not yet implemented"]
async fn test_touch_table_timestamp() {
    let _worker = test_worker();
    todo!("touch_table_timestamp 後に _tdw.table_timestamps を Query して行が存在することを検証する");
}

// ---------------------------------------------------------------------------
// TouchDbtTimestamps
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "dummy: not yet implemented"]
async fn test_touch_dbt_timestamps() {
    let _worker = test_worker();
    todo!("touch_dbt_timestamps 後に _tdw.table_timestamps を Query して複数行が存在することを検証する");
}
