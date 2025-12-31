# Fix D: フィールドタイプのenum化

## 問題
`MissingField` 構造体の `field_type` が文字列で管理されており、タイポや意図しない値が使われる危険性がある。

## 修正箇所
`src/main.rs` の `MissingField` 構造体とその使用箇所

## 修正1: FieldType enumの追加

```rust
/// 入力フィールドのタイプ
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    /// 日付入力
    Date,
    /// テキスト入力
    Text,
    /// 署名
    Signature,
    /// 選択肢
    Select,
    /// チェックボックス
    Checkbox,
}

impl FieldType {
    /// HTML input typeを取得
    pub fn input_type(&self) -> &'static str {
        match self {
            FieldType::Date => "date",
            FieldType::Text => "text",
            FieldType::Signature => "text", // 署名は別途処理
            FieldType::Select => "text",
            FieldType::Checkbox => "checkbox",
        }
    }

    /// プレースホルダーテキストを取得
    pub fn placeholder(&self) -> &'static str {
        match self {
            FieldType::Date => "YYYY-MM-DD",
            FieldType::Text => "入力してください",
            FieldType::Signature => "署名",
            FieldType::Select => "選択してください",
            FieldType::Checkbox => "",
        }
    }
}
```

## 修正2: MissingField構造体の更新

```rust
// 現在のコード
#[derive(Clone, Serialize, Deserialize)]
pub struct MissingField {
    pub field_name: String,
    pub field_type: String, // "date", "text", "signature"
    pub value: String,
    pub position: Option<FieldPosition>,
}

// 修正後
#[derive(Clone, Serialize, Deserialize)]
pub struct MissingField {
    pub field_name: String,
    pub field_type: FieldType,
    pub value: String,
    pub position: Option<FieldPosition>,
}
```

## 修正3: detect_missing_fields関数の更新

```rust
fn detect_missing_fields(ocr_result: &OcrResult) -> Vec<MissingField> {
    let mut missing = Vec::new();

    // 日付フィールドのチェック
    if !ocr_result.text.contains("令和") {
        missing.push(MissingField {
            field_name: "日付".to_string(),
            field_type: FieldType::Date,  // enumを使用
            value: String::new(),
            position: None,
        });
    }

    // 署名フィールドのチェック
    if !ocr_result.text.contains("印") {
        missing.push(MissingField {
            field_name: "代表者印".to_string(),
            field_type: FieldType::Signature,  // enumを使用
            value: String::new(),
            position: None,
        });
    }

    missing
}
```

## 修正4: フォーム表示での使用

```rust
// 現在のコード（文字列比較）
{fields.iter().map(|field| {
    let input_type = match field.field_type.as_str() {
        "date" => "date",
        "signature" => "text",
        _ => "text",
    };
    // ...
})}

// 修正後（enumメソッド使用）
{fields.iter().map(|field| {
    let input_type = field.field_type.input_type();
    let placeholder = field.field_type.placeholder();

    view! {
        <div class="field-input">
            <label>{&field.field_name}</label>
            <input
                type=input_type
                placeholder=placeholder
                // ...
            />
        </div>
    }
})}
```

## テスト方法
1. `cargo check` でコンパイルエラーがないことを確認
2. PDFビューワで不足フィールドが正しく表示されることを確認
3. 各フィールドタイプで適切な入力UIが表示されることを確認
