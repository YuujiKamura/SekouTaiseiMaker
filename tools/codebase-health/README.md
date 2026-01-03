# Codebase Health Dashboard

Rustで実装されたコードベース健全性分析ツール。プロジェクトのコード品質を解析し、オンラインのClaudeにタスクを分割して割り当てられる指示書を自動生成します。

## 機能

### 1. コードベース解析
- ファイル数・行数の統計（言語別）
- コード/コメント/空行の内訳
- テストファイルの検出

### 2. 複雑度分析
- 関数ごとの循環的複雑度（Cyclomatic Complexity）
- 最大ネスト深度
- 長い関数（50行超）の検出

### 3. 問題検出
- **セキュリティ**: ハードコードされた認証情報
- **コード品質**: `unwrap()`の使用、`any`型、console.log
- **パフォーマンス**: ループ内のclone()
- **保守性**: TODO/FIXME/HACKコメント
- **ベストプラクティス**: 言語固有のアンチパターン

### 4. Claude タスク指示書生成
- 問題を優先度付きタスクに変換
- 並列実行可能なタスクのグループ化
- 各タスクに対するコンテキスト、受け入れ基準、ヒントの提供

## インストール

```bash
cd tools/codebase-health
cargo build --release
```

## 使い方

### クイックサマリー
```bash
codebase-health summary --path /path/to/project
```

### 詳細レポート（Markdown形式）
```bash
codebase-health analyze --path /path/to/project --format markdown
```

### 詳細レポート（JSON形式）
```bash
codebase-health analyze --path /path/to/project --format json --output report.json
```

### Claudeタスク指示書の生成
```bash
codebase-health tasks --path /path/to/project --output-dir .claude/tasks
```

## 出力例

### サマリー出力
```
📊 Codebase Health Summary
========================

📁 Project: /home/user/SekouTaiseiMaker
📅 Analyzed: 2026-01-02 23:53:45 UTC

📈 Health Score: 75/100 ✅

📂 Files: 69 (15854 lines)
   Code: 12289 lines (77.5%)
   Comments: 1473 lines (9.3%)
   Blank: 2092 lines

📊 Languages:
   rs: 29 files, 5208 lines
   py: 14 files, 2335 lines
   tsx: 7 files, 2183 lines
   ts: 17 files, 1699 lines
   js: 2 files, 864 lines

⚠️  Issues: 374 total
   🔴 Critical: 2
   🟠 High: 1
   🟡 Medium: 43
   🟢 Low: 271
```

### 生成されるタスクファイル構造
```
.claude/tasks/
├── index.md           # タスク一覧
├── batch.md           # 並列実行用のグループ化
├── task-0001.md       # 個別タスク
├── task-0002.md
└── ...
```

### 個別タスクファイルの例
```markdown
# Task: task-0047

**Title:** Security improvements in pdf-editor.js
**Priority:** P1 (Critical)
**Category:** Security

## Files to Modify
- `/path/to/pdf-editor.js`

## Description
The following issues need to be addressed:
- **Hardcoded password** (line 1046)

## Acceptance Criteria
- [ ] All identified issues are resolved
- [ ] Use environment variables or a secure secrets manager
- [ ] Code compiles without errors
```

## オプション

### analyze コマンド
| オプション | 説明 | デフォルト |
|-----------|------|-----------|
| `--path, -p` | プロジェクトのルートパス | `.` |
| `--format, -f` | 出力形式 (markdown/json) | `markdown` |
| `--output, -o` | 出力ファイルパス | stdout |
| `--include-hidden` | 隠しファイルを含める | false |
| `--extensions, -e` | 対象拡張子（カンマ区切り） | `rs,ts,tsx,js,jsx,py,go,java` |

### tasks コマンド
| オプション | 説明 | デフォルト |
|-----------|------|-----------|
| `--path, -p` | プロジェクトのルートパス | `.` |
| `--output-dir, -o` | タスクファイルの出力先 | `.claude/tasks` |
| `--max-tasks-per-file` | ファイルあたりの最大タスク数 | `5` |
| `--priority-threshold` | 含める優先度の閾値 (1-5) | `3` |

## Claudeへのタスク割り当て方法

### 単一タスクの割り当て
```bash
claude-code "Complete task task-0001 following the instructions in .claude/tasks/task-0001.md"
```

### 並列タスクの割り当て
`batch.md`ファイルには、並列実行可能なタスクがグループ化されています。
異なるターミナルで同時に複数のClaudeインスタンスを起動できます：

```bash
# Terminal 1
claude-code "Complete task task-0001 following the instructions in .claude/tasks/task-0001.md"

# Terminal 2 (並列実行)
claude-code "Complete task task-0002 following the instructions in .claude/tasks/task-0002.md"
```

## 対応言語

- Rust (.rs)
- TypeScript (.ts, .tsx)
- JavaScript (.js, .jsx)
- Python (.py)
- Go (.go)
- Java (.java)

## ライセンス

MIT License
