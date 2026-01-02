//! OCR座標マッピングビュー
//!
//! Document AI OCRで検出したテキストの位置を可視化するビュー

use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

// ============================================
// OCRトークン可視化の型定義
// ============================================

/// OCRで検出されたテキストトークン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrToken {
    pub text: String,
    pub page: u32,
    pub normalized: NormalizedCoords,
    pub pixels: PixelCoords,
    pub page_size: PageSize,
}

/// 正規化された座標 (0.0〜1.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedCoords {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// ピクセル座標
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelCoords {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// ページサイズ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    pub width: f64,
    pub height: f64,
}

/// OCRドキュメント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrDocument {
    pub contractor: String,
    pub doc_type: String,
    pub image_url: String,
    pub tokens: Vec<OcrToken>,
}

// ============================================
// OCR可視化ビューのコンテキスト
// ============================================

/// OCRビューの状態管理コンテキスト
#[derive(Clone)]
pub struct OcrViewContext {
    pub documents: ReadSignal<Vec<OcrDocument>>,
    #[allow(dead_code)]
    pub set_documents: WriteSignal<Vec<OcrDocument>>,
    pub current_doc_index: ReadSignal<usize>,
    pub set_current_doc_index: WriteSignal<usize>,
    pub selected_token: ReadSignal<Option<usize>>,
    pub set_selected_token: WriteSignal<Option<usize>>,
    pub show_all_boxes: ReadSignal<bool>,
    pub set_show_all_boxes: WriteSignal<bool>,
}

// ============================================
// OCRビューアコンポーネント
// ============================================

/// OCR座標マッピングビュー
#[component]
pub fn OcrViewer() -> impl IntoView {
    let ctx = use_context::<OcrViewContext>().expect("OcrViewContext not found");

    view! {
        <div class="ocr-viewer">
            <div class="ocr-header">
                <h2>"OCR座標マッピング"</h2>
                <p class="ocr-description">
                    "Document AI OCRで検出したテキストの位置を表示します。"
                    <br/>
                    "緑枠: 検出されたテキスト / 赤枠: 選択中"
                </p>
            </div>

            // ドキュメント選択
            <div class="ocr-controls">
                <select on:change=move |ev| {
                    let idx: usize = event_target_value(&ev).parse().unwrap_or(0);
                    ctx.set_current_doc_index.set(idx);
                    ctx.set_selected_token.set(None);
                }>
                    {move || ctx.documents.get().iter().enumerate().map(|(i, doc)| {
                        view! {
                            <option value=i.to_string() selected=move || ctx.current_doc_index.get() == i>
                                {format!("{} - {}", doc.contractor, doc.doc_type)}
                            </option>
                        }
                    }).collect_view()}
                </select>

                <label class="checkbox-label">
                    <input type="checkbox"
                        prop:checked=move || ctx.show_all_boxes.get()
                        on:change=move |ev| ctx.set_show_all_boxes.set(event_target_checked(&ev))
                    />
                    "全ボックス表示"
                </label>
            </div>

            // Canvas表示エリア
            <div class="ocr-canvas-container">
                <OcrCanvas />
            </div>

            // トークン一覧
            <div class="ocr-token-list">
                <h4>"検出テキスト一覧"</h4>
                <div class="token-grid">
                    {move || {
                        let docs = ctx.documents.get();
                        let idx = ctx.current_doc_index.get();
                        if idx < docs.len() {
                            docs[idx].tokens.iter().enumerate().map(|(i, token)| {
                                let is_selected = ctx.selected_token.get() == Some(i);
                                let text = token.text.clone();
                                view! {
                                    <div
                                        class=format!("token-item {}", if is_selected { "selected" } else { "" })
                                        on:click=move |_| ctx.set_selected_token.set(Some(i))
                                    >
                                        <span class="token-text">{text}</span>
                                        <span class="token-coords">
                                            {format!("({:.0}, {:.0})", token.pixels.x, token.pixels.y)}
                                        </span>
                                    </div>
                                }
                            }).collect_view()
                        } else {
                            view! { <p>"ドキュメントがありません"</p> }.into_view()
                        }
                    }}
                </div>
            </div>

            // 選択中トークンの詳細
            {move || {
                let docs = ctx.documents.get();
                let doc_idx = ctx.current_doc_index.get();
                let token_idx = ctx.selected_token.get();

                if let (Some(doc), Some(t_idx)) = (docs.get(doc_idx), token_idx) {
                    if let Some(token) = doc.tokens.get(t_idx) {
                        Some(view! {
                            <div class="token-detail">
                                <h4>"選択中: \"" {token.text.clone()} "\""</h4>
                                <table>
                                    <tr><td>"正規化座標"</td><td>{format!("x: {:.4}, y: {:.4}", token.normalized.x, token.normalized.y)}</td></tr>
                                    <tr><td>"サイズ"</td><td>{format!("w: {:.4}, h: {:.4}", token.normalized.width, token.normalized.height)}</td></tr>
                                    <tr><td>"ピクセル座標"</td><td>{format!("x: {}, y: {}", token.pixels.x, token.pixels.y)}</td></tr>
                                    <tr><td>"ピクセルサイズ"</td><td>{format!("w: {}, h: {}", token.pixels.width, token.pixels.height)}</td></tr>
                                </table>
                            </div>
                        })
                    } else { None }
                } else { None }
            }}
        </div>
    }
}

// ============================================
// OCR Canvas コンポーネント
// ============================================

/// OCRトークンを描画するCanvas
#[component]
pub fn OcrCanvas() -> impl IntoView {
    let ctx = use_context::<OcrViewContext>().expect("OcrViewContext not found");
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // 読み込み済み画像を保持するシグナル
    let (loaded_image, set_loaded_image) = create_signal::<Option<HtmlImageElement>>(None);
    // 現在読み込み中の画像URL
    let (loading_url, set_loading_url) = create_signal::<String>(String::new());

    // 画像読み込みエフェクト
    create_effect(move |_| {
        let docs = ctx.documents.get();
        let doc_idx = ctx.current_doc_index.get();

        if let Some(doc) = docs.get(doc_idx) {
            let image_url = doc.image_url.clone();

            // 新しい画像URLなら読み込み開始
            if !image_url.is_empty() && image_url != loading_url.get_untracked() {
                set_loading_url.set(image_url.clone());
                set_loaded_image.set(None);

                // 画像エレメントを作成
                if let Ok(img) = HtmlImageElement::new() {
                    let _set_img = set_loaded_image.clone();

                    // onloadコールバック
                    let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                        // 画像読み込み完了 - 再描画トリガー
                    }) as Box<dyn FnMut(_)>);

                    img.set_onload(Some(onload.as_ref().unchecked_ref()));
                    onload.forget();

                    img.set_src(&image_url);
                    set_loaded_image.set(Some(img));
                }
            }
        }
    });

    // Canvas描画エフェクト
    create_effect(move |_| {
        let docs = ctx.documents.get();
        let doc_idx = ctx.current_doc_index.get();
        let show_all = ctx.show_all_boxes.get();
        let selected = ctx.selected_token.get();
        let img = loaded_image.get();

        if let Some(doc) = docs.get(doc_idx) {
            if let Some(canvas) = canvas_ref.get() {
                let canvas_el: &HtmlCanvasElement = &canvas;
                draw_ocr_canvas(canvas_el, doc, show_all, selected, img.as_ref());
            }
        }
    });

    view! {
        <canvas
            node_ref=canvas_ref
            class="ocr-canvas"
            width="800"
            height="1130"
        />
    }
}

// ============================================
// 描画関数
// ============================================

/// CanvasにOCRトークンを描画
fn draw_ocr_canvas(
    canvas: &HtmlCanvasElement,
    doc: &OcrDocument,
    show_all: bool,
    selected: Option<usize>,
    background_img: Option<&HtmlImageElement>,
) {
    let ctx = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok());

    if let Some(ctx) = ctx {
        let canvas_width = canvas.width() as f64;
        let canvas_height = canvas.height() as f64;

        // 背景クリア
        ctx.set_fill_style_str("#f5f5f5");
        ctx.fill_rect(0.0, 0.0, canvas_width, canvas_height);

        // ページサイズを取得（最初のトークンから）
        let page_size = doc
            .tokens
            .first()
            .map(|t| (t.page_size.width, t.page_size.height))
            .unwrap_or((1681.0, 2378.0));

        // スケール計算
        let scale_x = canvas_width / page_size.0;
        let scale_y = canvas_height / page_size.1;
        let scale = scale_x.min(scale_y);

        // オフセット（センタリング）
        let offset_x = (canvas_width - page_size.0 * scale) / 2.0;
        let offset_y = (canvas_height - page_size.1 * scale) / 2.0;

        // 背景画像を描画（ある場合）
        if let Some(img) = background_img {
            if img.complete() && img.natural_width() > 0 {
                let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(
                    img,
                    offset_x,
                    offset_y,
                    page_size.0 * scale,
                    page_size.1 * scale,
                );
            } else {
                ctx.set_fill_style_str("#ffffff");
                ctx.fill_rect(offset_x, offset_y, page_size.0 * scale, page_size.1 * scale);
            }
        } else {
            ctx.set_fill_style_str("#ffffff");
            ctx.fill_rect(offset_x, offset_y, page_size.0 * scale, page_size.1 * scale);
        }

        // ページ境界線
        ctx.set_stroke_style_str("#cccccc");
        ctx.set_line_width(1.0);
        ctx.stroke_rect(offset_x, offset_y, page_size.0 * scale, page_size.1 * scale);

        // トークンを描画
        for (i, token) in doc.tokens.iter().enumerate() {
            let is_selected = selected == Some(i);
            let is_marker = token.text == "御"
                || token.text == "中"
                || token.text == "令"
                || token.text == "和"
                || token.text == "年"
                || token.text == "月"
                || token.text == "日"
                || token.text == "殿"
                || token.text == "様";

            // 表示するかどうか
            if !show_all && !is_selected && !is_marker {
                continue;
            }

            let x = offset_x + token.normalized.x * page_size.0 * scale;
            let y = offset_y + token.normalized.y * page_size.1 * scale;
            let w = token.normalized.width * page_size.0 * scale;
            let h = token.normalized.height * page_size.1 * scale;

            // 色設定
            let (stroke_color, fill_color, line_width) = if is_selected {
                ("#ff0000", "rgba(255, 0, 0, 0.2)", 3.0) // 赤: 選択中
            } else if is_marker {
                ("#0066ff", "rgba(0, 102, 255, 0.15)", 2.0) // 青: マーカー
            } else {
                ("#00aa00", "rgba(0, 170, 0, 0.1)", 1.0) // 緑: 通常
            };

            // 塗りつぶし
            ctx.set_fill_style_str(fill_color);
            ctx.fill_rect(x, y, w, h);

            // 枠線
            ctx.set_stroke_style_str(stroke_color);
            ctx.set_line_width(line_width);
            ctx.stroke_rect(x, y, w, h);

            // テキストラベル（マーカーまたは選択中のみ）
            if is_selected || is_marker {
                ctx.set_fill_style_str(stroke_color);
                ctx.set_font("12px sans-serif");
                let _ = ctx.fill_text(&token.text, x, y - 2.0);
            }
        }

        // 凡例
        ctx.set_font("14px sans-serif");
        ctx.set_fill_style_str("#333333");
        let _ = ctx.fill_text("凡例:", 10.0, 20.0);

        ctx.set_fill_style_str("#0066ff");
        let _ = ctx.fill_text("■ マーカー(御/令和/年月日)", 10.0, 40.0);

        ctx.set_fill_style_str("#00aa00");
        let _ = ctx.fill_text("■ 通常テキスト", 10.0, 60.0);

        ctx.set_fill_style_str("#ff0000");
        let _ = ctx.fill_text("■ 選択中", 10.0, 80.0);
    }
}
