use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::db::types::TableMeta;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColumnMetaYml {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TableMetaYml {
    pub name: String,
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub columns: Vec<ColumnMetaYml>,
}

pub fn yml_path(home: &Path, schema_name: &str, table_name: &str) -> PathBuf {
    home.join(".tdwh")
        .join("metadata")
        .join("adhoc")
        .join(schema_name)
        .join(format!("{}.yml", table_name))
}

pub fn write_table_meta_yml(home: &Path, meta: &TableMeta) -> Result<(), String> {
    let path = yml_path(home, &meta.schema_name, &meta.table_name);
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;

    let yml_data = TableMetaYml {
        name: meta.table_name.clone(),
        schema: meta.schema_name.clone(),
        description: meta.comment.clone(),
        columns: meta.columns.iter().map(|c| ColumnMetaYml {
            name: c.name.clone(),
            data_type: c.data_type.clone(),
            description: c.comment.clone(),
        }).collect(),
    };

    let content = serde_yaml::to_string(&yml_data).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn read_all_ymls(home: &Path) -> Vec<TableMetaYml> {
    let base = home.join(".tdwh").join("metadata").join("adhoc");
    let mut results = Vec::new();

    let Ok(schemas) = std::fs::read_dir(&base) else { return results; };
    for schema_entry in schemas.flatten() {
        if !schema_entry.file_type().map_or(false, |t| t.is_dir()) { continue; }
        let Ok(tables) = std::fs::read_dir(schema_entry.path()) else { continue; };
        for table_entry in tables.flatten() {
            let path = table_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yml") { continue; }
            let Ok(content) = std::fs::read_to_string(&path) else { continue; };
            let Ok(yml) = serde_yaml::from_str::<TableMetaYml>(&content) else { continue; };
            results.push(yml);
        }
    }
    results
}
