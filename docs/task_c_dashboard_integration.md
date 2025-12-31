# Task C: ダッシュボード連携

## 目的
ダッシュボードの書類セルをクリック可能にし、適切なビューワを起動する。

## 前提条件
- Task A (PDFビューワ) 完了
- Task B (スプレッドシートビューワ) 完了

## 技術スタック
- Rust / Leptos 0.6 (CSR mode)

## 修正箇所

### 1. ViewMode enum の拡張

```rust
#[derive(Clone, PartialEq)]
pub enum ViewMode {
    Dashboard,
    OcrViewer,
    PdfViewer { contractor: String, doc_type: String, url: String },
    SpreadsheetViewer { contractor: String, doc_type: String, url: String },
}
```

### 2. 書類タイプ判定関数

```rust
#[derive(Clone, PartialEq)]
pub enum DocFileType {
    Pdf,
    GoogleSpreadsheet,
    Excel,
    GoogleDoc,
    Image,
    Unknown,
}

fn detect_file_type(url: &str) -> DocFileType {
    let url_lower = url.to_lowercase();

    if url_lower.contains("docs.google.com/spreadsheets") {
        DocFileType::GoogleSpreadsheet
    } else if url_lower.contains("docs.google.com/document") {
        DocFileType::GoogleDoc
    } else if url_lower.contains("drive.google.com/file") {
        // Google DriveのファイルはデフォルトでPDF扱い
        // 実際にはAPIでMIMEタイプを確認すべき
        DocFileType::Pdf
    } else if url_lower.ends_with(".pdf") {
        DocFileType::Pdf
    } else if url_lower.ends_with(".xlsx") || url_lower.ends_with(".xls") {
        DocFileType::Excel
    } else if url_lower.ends_with(".png") || url_lower.ends_with(".jpg") || url_lower.ends_with(".jpeg") {
        DocFileType::Image
    } else {
        DocFileType::Unknown
    }
}
```

### 3. ContractorRow コンポーネントの修正

現在の書類セル表示を修正して、クリック可能にする:

```rust
// 現在のコード（ステータスアイコン表示のみ）
view! {
    <td class="status-cell">
        {status_icon}
    </td>
}

// 修正後（クリックでビューワ起動）
view! {
    <td
        class="status-cell clickable"
        on:click=move |_| {
            if let Some(url) = doc_url.clone() {
                let file_type = detect_file_type(&url);
                match file_type {
                    DocFileType::Pdf | DocFileType::Image => {
                        set_view_mode.set(ViewMode::PdfViewer {
                            contractor: contractor_name.clone(),
                            doc_type: doc_type.clone(),
                            url: url,
                        });
                    }
                    DocFileType::GoogleSpreadsheet | DocFileType::Excel => {
                        set_view_mode.set(ViewMode::SpreadsheetViewer {
                            contractor: contractor_name.clone(),
                            doc_type: doc_type.clone(),
                            url: url,
                        });
                    }
                    _ => {
                        // 不明な場合はURLを新規タブで開く
                        if let Some(window) = web_sys::window() {
                            let _ = window.open_with_url_and_target(&url, "_blank");
                        }
                    }
                }
            }
        }
    >
        {status_icon}
        <span class="click-hint">"クリックで開く"</span>
    </td>
}
```

### 4. メインビュー分岐の修正

```rust
#[component]
fn App() -> impl IntoView {
    let (view_mode, set_view_mode) = create_signal(ViewMode::Dashboard);

    view! {
        <div class="app-container">
            {move || match view_mode.get() {
                ViewMode::Dashboard => view! {
                    <Dashboard set_view_mode=set_view_mode />
                }.into_view(),

                ViewMode::OcrViewer => view! {
                    <OcrViewer set_view_mode=set_view_mode />
                }.into_view(),

                ViewMode::PdfViewer { contractor, doc_type, url } => view! {
                    <PdfViewer
                        contractor=contractor
                        doc_type=doc_type
                        url=url
                        set_view_mode=set_view_mode
                    />
                }.into_view(),

                ViewMode::SpreadsheetViewer { contractor, doc_type, url } => view! {
                    <SpreadsheetViewer
                        contractor=contractor
                        doc_type=doc_type
                        url=url
                        set_view_mode=set_view_mode
                    />
                }.into_view(),
            }}
        </div>
    }
}
```

## CSS追加

```css
.status-cell.clickable {
    cursor: pointer;
    position: relative;
}

.status-cell.clickable:hover {
    background-color: #e3f2fd;
}

.status-cell.clickable:hover .click-hint {
    opacity: 1;
}

.click-hint {
    position: absolute;
    bottom: 2px;
    left: 50%;
    transform: translateX(-50%);
    font-size: 10px;
    color: #666;
    opacity: 0;
    transition: opacity 0.2s;
    white-space: nowrap;
}

/* 共通ヘッダー */
.viewer-header {
    display: flex;
    align-items: center;
    padding: 15px 20px;
    background: #fff;
    border-bottom: 1px solid #ddd;
}

.viewer-header .back-button {
    padding: 8px 16px;
    background: #f5f5f5;
    border: 1px solid #ddd;
    border-radius: 4px;
    cursor: pointer;
    margin-right: 20px;
}

.viewer-header .back-button:hover {
    background: #e0e0e0;
}

.viewer-header .doc-title {
    flex: 1;
    font-size: 18px;
    font-weight: bold;
}
```

## テストシナリオ

1. ダッシュボードで「暴対法誓約書」（PDF）をクリック → PDFビューワが開く
2. ダッシュボードで「作業員名簿」（スプレッドシート）をクリック → スプレッドシートビューワが開く
3. 各ビューワの「戻る」ボタン → ダッシュボードに戻る
4. URLがない書類セルをクリック → 何も起きない

## 注意事項
- 既存のContractorRowコンポーネントの構造を理解してから修正
- set_view_modeシグナルをpropsとして渡す必要あり
