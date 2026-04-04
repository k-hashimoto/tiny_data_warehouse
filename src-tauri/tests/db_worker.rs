use tiny_data_ware_house_lib::db::worker::DbWorker;

fn test_worker() -> DbWorker {
    DbWorker::new(":memory:", "")
}

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

#[tokio::test]
async fn test_get_schema() {
    let worker = test_worker();
    worker.query("CREATE TABLE bar (id INTEGER, label VARCHAR)".to_string()).await.unwrap();
    let schema = worker.get_schema("main".to_string(), "bar".to_string()).await.unwrap();
    let col_names: Vec<&str> = schema.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"id"));
    assert!(col_names.contains(&"label"));
}

#[tokio::test]
async fn test_query_truncated_flag() {
    let worker = test_worker();
    worker.query("CREATE TABLE big AS SELECT range AS id FROM range(1001)".to_string()).await.unwrap();
    let result = worker.query("SELECT * FROM big".to_string()).await.unwrap();
    assert!(result.truncated);
    assert_eq!(result.rows.len(), 1000);
}
