use crate::db::types::TableMeta;
use crate::db::worker::DbWorker;
use crate::metadata_yml;
use crate::utils;
use serde::Deserialize;
use tauri::{Manager, State};

#[tauri::command]
pub async fn get_table_meta(
    schema_name: String,
    table_name: String,
    db: State<'_, DbWorker>,
) -> Result<TableMeta, String> {
    db.get_table_meta(schema_name, table_name).await
}

#[tauri::command]
pub async fn get_dbt_table_meta(
    schema_name: String,
    table_name: String,
    db: State<'_, DbWorker>,
) -> Result<TableMeta, String> {
    let path = utils::dbt_db_path();
    if !path.exists() {
        return Err("dbt.db not found".into());
    }
    db.get_dbt_table_meta(
        path.to_str().unwrap_or("").to_string(),
        schema_name,
        table_name,
    )
    .await
}

#[tauri::command]
pub async fn set_table_comment(
    schema_name: String,
    table_name: String,
    comment: String,
    db: State<'_, DbWorker>,
) -> Result<(), String> {
    db.set_table_comment(schema_name, table_name, comment).await
}

#[tauri::command]
pub async fn set_column_comment(
    schema_name: String,
    table_name: String,
    column_name: String,
    comment: String,
    db: State<'_, DbWorker>,
) -> Result<(), String> {
    db.set_column_comment(schema_name, table_name, column_name, comment)
        .await
}

#[derive(Deserialize)]
pub struct ColCommentUpdate {
    pub name: String,
    pub comment: String,
}

/// テーブルメタデータを保存: DuckDB COMMENT + ~/.tdwh/metadata/adhoc/<schema>/<table>.yml
#[tauri::command]
pub async fn save_table_meta(
    app: tauri::AppHandle,
    schema_name: String,
    table_name: String,
    comment: String,
    col_comments: Vec<ColCommentUpdate>,
    db: State<'_, DbWorker>,
) -> Result<(), String> {
    // DuckDB コメントを更新
    db.set_table_comment(schema_name.clone(), table_name.clone(), comment)
        .await?;
    for col in &col_comments {
        db.set_column_comment(
            schema_name.clone(),
            table_name.clone(),
            col.name.clone(),
            col.comment.clone(),
        )
        .await?;
    }
    // updated_at を更新
    let _ = db
        .touch_table_timestamp(
            schema_name.clone(),
            table_name.clone(),
            "adhoc".to_string(),
            false,
        )
        .await;
    // 更新後のメタ（データ型付き）を取得して yml に書き込む
    let meta = db.get_table_meta(schema_name, table_name).await?;
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    metadata_yml::write_table_meta_yml(&home, &meta)?;
    Ok(())
}

/// dbt テーブルの updated_at を一括更新（dbt run 後に呼ぶ）
#[tauri::command]
pub async fn touch_dbt_timestamps(
    tables: Vec<(String, String)>,
    db: State<'_, DbWorker>,
) -> Result<(), String> {
    db.touch_dbt_timestamps(tables).await
}

/// 起動時に ~/.tdwh/metadata/adhoc/ の yml を DuckDB COMMENT に同期
#[tauri::command]
pub async fn sync_yml_metadata(
    app: tauri::AppHandle,
    db: State<'_, DbWorker>,
) -> Result<(), String> {
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let ymls = metadata_yml::read_all_ymls(&home);
    for yml in ymls {
        let _ = db
            .set_table_comment(
                yml.schema.clone(),
                yml.name.clone(),
                yml.description.clone().unwrap_or_default(),
            )
            .await;
        for col in &yml.columns {
            if let Some(desc) = &col.description {
                let _ = db
                    .set_column_comment(
                        yml.schema.clone(),
                        yml.name.clone(),
                        col.name.clone(),
                        desc.clone(),
                    )
                    .await;
            }
        }
    }
    Ok(())
}
