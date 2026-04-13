# イシュー起点の開発手順

`/start-issue` でイシューから開発を開始する場合、以下の手順に従う。

## ステアリングファイルが存在する場合

イシューに `.steering/` ディレクトリへの参照が含まれる、またはステアリングファイルが既に存在する場合：

1. 対象の `.steering/[ディレクトリ]/tasklist.md` を確認する
2. 実施するタスク（例：T2, T3）に対応するチェックリストを特定する
3. **Codex MCP (`mcp__codex__codex`) を呼び出して実装を委譲する**
   - プランナーフェーズは省略してよい（ステアリングファイルが設計を代替するため）
   - 呼び出しパラメータ：
     - `prompt`: `tasklist.md` と `design.md` の内容を含む実装指示
     - `cwd`: プロジェクトルートの絶対パス
     - `sandbox`: `workspace-write`
     - `approval-policy`: `on-failure`
     - `base-instructions`: `docs_for_ai/development-guidelines.md` の内容
4. Codex完了後に `cargo build` / `cargo test` / `cargo clippy` を確認する
5. 追加指示が必要な場合は `mcp__codex__codex-reply` で `threadId` を使って継続する

## ステアリングファイルが存在しない場合

通常の開発プロセス（planner → 承認 → Codex MCP）に従う。
