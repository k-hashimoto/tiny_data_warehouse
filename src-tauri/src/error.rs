/// アプリケーション共通エラー型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("DB エラー: {0}")]
    Db(#[from] duckdb::Error),

    #[error("IO エラー: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML エラー: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("AI操作中です。しばらく待ってから再試行してください。")]
    McpLocked,

    #[error("{0}")]
    Other(String),
}

/// Tauri コマンド用 Serialize 実装（Tauri は serde::Serialize が必要）
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
