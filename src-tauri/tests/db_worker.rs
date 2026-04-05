use tiny_data_ware_house_lib::db::types::{CsvImportOptions, JsonImportOptions};
use tiny_data_ware_house_lib::db::worker::DbWorker;
use tiny_data_ware_house_lib::scheduler::{JobType, ScheduledJob};

fn test_worker() -> DbWorker {
    DbWorker::new(":memory:", "")
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

mod query {
    use super::*;

    // 単純なSELECT文を実行したとき、カラム名と行データが正しく返ることを確認する
    #[tokio::test]
    async fn test_query_basic() {
        let worker = test_worker();
        let result = worker.query("SELECT 42 AS answer".to_string()).await.unwrap();
        assert_eq!(result.columns, vec!["answer"]);
        assert_eq!(result.rows[0][0], serde_json::json!(42));
    }

    // 不正なSQL文を実行したとき、Err が返ることを確認する
    #[tokio::test]
    async fn test_query_invalid_sql_returns_error() {
        let worker = test_worker();
        let result = worker.query("NOT VALID SQL".to_string()).await;
        assert!(result.is_err());
    }

    // 1001行のテーブルを全件取得したとき、rows が1000件に切り捨てられ truncated=true になることを確認する
    #[tokio::test]
    async fn test_query_truncated_flag() {
        let worker = test_worker();
        worker.query("CREATE TABLE big AS SELECT range AS id FROM range(1001)".to_string()).await.unwrap();
        let result = worker.query("SELECT * FROM big".to_string()).await.unwrap();
        assert!(result.truncated);
        assert_eq!(result.rows.len(), 1000);
    }
}

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

mod tables {
    use super::*;

    // 何もテーブルを作っていないインメモリDBでは、list_tables が空リストを返すことを確認する
    #[tokio::test]
    async fn test_list_tables_initially_empty() {
        let worker = test_worker();
        let tables = worker.list_tables().await.unwrap();
        assert!(tables.is_empty());
    }

    // テーブルを作成した後、list_tables の結果にそのテーブル名が含まれることを確認する
    #[tokio::test]
    async fn test_list_tables_after_create() {
        let worker = test_worker();
        worker.query("CREATE TABLE foo (id INTEGER, name VARCHAR)".to_string()).await.unwrap();
        let tables = worker.list_tables().await.unwrap();
        assert!(tables.iter().any(|t| t.name == "foo"));
    }

    // テーブル作成後に get_schema を呼び、定義したカラム名がスキーマ情報に含まれることを確認する
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
    async fn test_preview_table() {
        let worker = test_worker();
        worker.query("CREATE TABLE t (id INTEGER, name VARCHAR, value INTEGER)".to_string()).await.unwrap();
        worker.query("INSERT INTO t VALUES (1, 'Alice', 100), (2, 'Bob', 200), (3, 'Carol', 300)".to_string()).await.unwrap();
        let result = worker.preview_table("t".to_string(), 100).await.unwrap();
        assert_eq!(result.rows.len(), 3);
    }

    #[tokio::test]
    async fn test_list_schemas() {
        let worker = test_worker();
        let schemas = worker.list_schemas().await.unwrap();
        assert!(schemas.contains(&"main".to_string()));
    }
}

// ---------------------------------------------------------------------------
// Csv
// ---------------------------------------------------------------------------

mod csv {
    use super::*;

    #[tokio::test]
    async fn test_preview_csv() {
        let worker = test_worker();
        let csv_path = format!("{}/tests/fixtures/sample.csv", env!("CARGO_MANIFEST_DIR"));
        let opts = CsvImportOptions {
            file_path: csv_path,
            table_name: "preview_test".to_string(),
            schema_name: "main".to_string(),
            has_header: true,
            delimiter: ",".to_string(),
            encoding: "utf8".to_string(),
            if_exists: "replace".to_string(),
        };
        let result = worker.preview_csv(opts).await.unwrap();
        assert_eq!(result.preview.columns, vec!["id", "name", "value"]);
        assert_eq!(result.preview.rows.len(), 3);
        assert_eq!(result.suggested_table_name, "sample");
    }

    #[tokio::test]
    async fn test_import_csv() {
        let csv_path = format!("{}/tests/fixtures/sample.csv", env!("CARGO_MANIFEST_DIR"));

        // replace モード: テーブルが作成され row_count が CSV の行数と一致する
        {
            let worker = test_worker();
            let opts = CsvImportOptions {
                file_path: csv_path.clone(),
                table_name: "sales".to_string(),
                schema_name: "main".to_string(),
                has_header: true,
                delimiter: ",".to_string(),
                encoding: "utf8".to_string(),
                if_exists: "replace".to_string(),
            };
            let result = worker.import_csv(opts).await.unwrap();
            assert_eq!(result.row_count, 3);
            assert_eq!(result.name, "sales");

            // append モード: 既存テーブルに行が追加される
            let opts_append = CsvImportOptions {
                file_path: csv_path.clone(),
                table_name: "sales".to_string(),
                schema_name: "main".to_string(),
                has_header: true,
                delimiter: ",".to_string(),
                encoding: "utf8".to_string(),
                if_exists: "append".to_string(),
            };
            let result_append = worker.import_csv(opts_append).await.unwrap();
            assert_eq!(result_append.row_count, 6);
        }

        // fail モード: テーブルが既に存在する場合に Err が返る
        {
            let worker = test_worker();
            // まず replace でテーブルを作成する
            let opts_replace = CsvImportOptions {
                file_path: csv_path.clone(),
                table_name: "sales".to_string(),
                schema_name: "main".to_string(),
                has_header: true,
                delimiter: ",".to_string(),
                encoding: "utf8".to_string(),
                if_exists: "replace".to_string(),
            };
            worker.import_csv(opts_replace).await.unwrap();

            // fail モードで再度インポートすると Err が返る
            let opts_fail = CsvImportOptions {
                file_path: csv_path.clone(),
                table_name: "sales".to_string(),
                schema_name: "main".to_string(),
                has_header: true,
                delimiter: ",".to_string(),
                encoding: "utf8".to_string(),
                if_exists: "fail".to_string(),
            };
            let result_fail = worker.import_csv(opts_fail).await;
            assert!(result_fail.is_err());
        }
    }

    #[tokio::test]
    async fn test_reimport_csv() {
        let worker = test_worker();
        let csv_path = format!("{}/tests/fixtures/sample.csv", env!("CARGO_MANIFEST_DIR"));

        // replace モードで import_csv を呼ぶ
        let opts = CsvImportOptions {
            file_path: csv_path,
            table_name: "sales".to_string(),
            schema_name: "main".to_string(),
            has_header: true,
            delimiter: ",".to_string(),
            encoding: "utf8".to_string(),
            if_exists: "replace".to_string(),
        };
        worker.import_csv(opts).await.unwrap();

        // reimport_csv を呼んで再インポートされることを確認する
        let result = worker.reimport_csv("main".to_string(), "sales".to_string()).await.unwrap();
        assert_eq!(result.row_count, 3);
    }
}

// ---------------------------------------------------------------------------
// Json
// ---------------------------------------------------------------------------

mod json {
    use super::*;

    #[tokio::test]
    async fn test_preview_json() {
        let worker = test_worker();
        let json_path = format!("{}/tests/fixtures/sample.json", env!("CARGO_MANIFEST_DIR"));
        let opts = JsonImportOptions {
            file_path: json_path,
            table_name: "sample".to_string(),
            schema_name: "main".to_string(),
            if_exists: "replace".to_string(),
        };
        let result = worker.preview_json(opts).await.unwrap();
        assert_eq!(result.preview.rows.len(), 3);
        assert_eq!(result.suggested_table_name, "sample");
    }

    #[tokio::test]
    async fn test_import_json() {
        let worker = test_worker();
        let json_path = format!("{}/tests/fixtures/sample.json", env!("CARGO_MANIFEST_DIR"));
        let opts = JsonImportOptions {
            file_path: json_path,
            table_name: "sample".to_string(),
            schema_name: "main".to_string(),
            if_exists: "replace".to_string(),
        };
        let result = worker.import_json(opts).await.unwrap();
        assert_eq!(result.row_count, 3);
    }
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

mod metadata {
    use super::*;

    #[tokio::test]
    async fn test_get_table_meta() {
        let worker = test_worker();
        worker.query("CREATE TABLE meta_test (id INTEGER, name VARCHAR)".to_string()).await.unwrap();
        let meta = worker.get_table_meta("main".to_string(), "meta_test".to_string()).await.unwrap();
        assert_eq!(meta.schema_name, "main");
        assert_eq!(meta.table_name, "meta_test");
    }

    #[tokio::test]
    async fn test_set_table_comment() {
        let worker = test_worker();
        worker.query("CREATE TABLE comment_test (id INTEGER)".to_string()).await.unwrap();
        worker.set_table_comment("main".to_string(), "comment_test".to_string(), "テストコメント".to_string()).await.unwrap();
        let meta = worker.get_table_meta("main".to_string(), "comment_test".to_string()).await.unwrap();
        assert_eq!(meta.comment, Some("テストコメント".to_string()));
    }

    #[tokio::test]
    async fn test_set_column_comment() {
        let worker = test_worker();
        worker.query("CREATE TABLE col_comment_test (id INTEGER, name VARCHAR)".to_string()).await.unwrap();
        worker.set_column_comment("main".to_string(), "col_comment_test".to_string(), "id".to_string(), "IDカラム".to_string()).await.unwrap();
        let meta = worker.get_table_meta("main".to_string(), "col_comment_test".to_string()).await.unwrap();
        let id_col = meta.columns.iter().find(|c| c.name == "id").unwrap();
        assert_eq!(id_col.comment, Some("IDカラム".to_string()));
    }
}

// ---------------------------------------------------------------------------
// Timestamp
// ---------------------------------------------------------------------------

mod timestamp {
    use super::*;

    #[tokio::test]
    async fn test_touch_table_timestamp() {
        let worker = test_worker();
        worker.touch_table_timestamp("main".to_string(), "ts_test".to_string(), "adhoc".to_string(), true).await.unwrap();
        let result = worker.query("SELECT COUNT(*) FROM _tdw.table_timestamps WHERE schema_name = 'main' AND table_name = 'ts_test'".to_string()).await.unwrap();
        assert_eq!(result.rows[0][0], serde_json::json!(1));
    }

    #[tokio::test]
    async fn test_touch_dbt_timestamps() {
        let worker = test_worker();
        let tables = vec![
            ("main".to_string(), "a".to_string()),
            ("main".to_string(), "b".to_string()),
        ];
        worker.touch_dbt_timestamps(tables).await.unwrap();
        let result = worker.query("SELECT COUNT(*) FROM _tdw.table_timestamps WHERE source = 'dbt'".to_string()).await.unwrap();
        assert_eq!(result.rows[0][0], serde_json::json!(2));
    }
}

// ---------------------------------------------------------------------------
// Dbt
// ---------------------------------------------------------------------------

mod dbt {
    use super::*;

    #[tokio::test]
    #[ignore = "dummy: dbt DB fixture not yet implemented"]
    async fn test_list_dbt_tables() {
        let _worker = test_worker();
        todo!("dbt DB fixture が用意できたら実装する");
    }

    #[tokio::test]
    #[ignore = "dummy: dbt DB fixture not yet implemented"]
    async fn test_preview_dbt_table() {
        let _worker = test_worker();
        todo!("dbt DB fixture が用意できたら実装する");
    }

    #[tokio::test]
    #[ignore = "dummy: dbt DB fixture not yet implemented"]
    async fn test_attach_and_detach_dbt() {
        let _worker = test_worker();
        todo!("dbt DB ファイルを用意して attach_dbt / detach_dbt を検証する");
    }

    #[tokio::test]
    #[ignore = "dummy: dbt DB fixture not yet implemented"]
    async fn test_get_dbt_schema() {
        let _worker = test_worker();
        todo!("dbt DB fixture が用意できたら実装する");
    }

    #[tokio::test]
    #[ignore = "dummy: dbt DB fixture not yet implemented"]
    async fn test_drop_dbt_table() {
        let _worker = test_worker();
        todo!("dbt DB fixture が用意できたら実装する");
    }

    #[tokio::test]
    #[ignore = "dummy: dbt DB fixture not yet implemented"]
    async fn test_drop_dbt_schema() {
        let _worker = test_worker();
        todo!("dbt DB fixture が用意できたら実装する");
    }

    #[tokio::test]
    #[ignore = "dummy: dbt DB fixture not yet implemented"]
    async fn test_get_dbt_table_meta() {
        let _worker = test_worker();
        todo!("dbt DB fixture が用意できたら実装する");
    }
}

// ---------------------------------------------------------------------------
// Scheduled Jobs
// ---------------------------------------------------------------------------

mod scheduled_jobs {
    use super::*;

    fn make_job(id: &str, name: &str) -> ScheduledJob {
        ScheduledJob {
            id: id.to_string(),
            name: name.to_string(),
            job_type: JobType::Query,
            target_id: "scripts/my_query.sql".to_string(),
            cron_expr: "0 * * * *".to_string(),
            enabled: true,
            created_at: "2026-01-01 00:00:00".to_string(),
            last_run_at: None,
        }
    }

    // 初期状態では scheduled_jobs が空であることを確認する
    #[tokio::test]
    async fn test_list_scheduled_jobs_initially_empty() {
        let worker = test_worker();
        let jobs = worker.list_scheduled_jobs().await.unwrap();
        assert!(jobs.is_empty());
    }

    // ジョブを保存すると list で取得できることを確認する
    #[tokio::test]
    async fn test_save_and_list_scheduled_job() {
        let worker = test_worker();
        let job = make_job("job-001", "Daily Query");
        worker.save_scheduled_job(job.clone()).await.unwrap();

        let jobs = worker.list_scheduled_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "job-001");
        assert_eq!(jobs[0].name, "Daily Query");
        assert_eq!(jobs[0].cron_expr, "0 * * * *");
        assert!(jobs[0].enabled);
    }

    // 同じ id で保存すると upsert されることを確認する
    #[tokio::test]
    async fn test_save_scheduled_job_upsert() {
        let worker = test_worker();
        let job = make_job("job-002", "Original Name");
        worker.save_scheduled_job(job).await.unwrap();

        let updated = ScheduledJob {
            id: "job-002".to_string(),
            name: "Updated Name".to_string(),
            job_type: JobType::Import,
            target_id: "my_table".to_string(),
            cron_expr: "30 6 * * *".to_string(),
            enabled: false,
            created_at: "2026-01-01 00:00:00".to_string(),
            last_run_at: Some("2026-01-02 06:30:00".to_string()),
        };
        worker.save_scheduled_job(updated).await.unwrap();

        let jobs = worker.list_scheduled_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "Updated Name");
        assert!(!jobs[0].enabled);
    }

    // ジョブを削除すると list から消えることを確認する
    #[tokio::test]
    async fn test_delete_scheduled_job() {
        let worker = test_worker();
        worker.save_scheduled_job(make_job("job-003", "A")).await.unwrap();
        worker.save_scheduled_job(make_job("job-004", "B")).await.unwrap();

        worker.delete_scheduled_job("job-003".to_string()).await.unwrap();

        let jobs = worker.list_scheduled_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "job-004");
    }

    // 存在しない id を削除してもエラーにならないことを確認する
    #[tokio::test]
    async fn test_delete_nonexistent_job_is_ok() {
        let worker = test_worker();
        let result = worker.delete_scheduled_job("nonexistent".to_string()).await;
        assert!(result.is_ok());
    }
}
