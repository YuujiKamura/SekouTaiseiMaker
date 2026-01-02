/**
 * ブラウザで動的にフォントサブセットを作成するユーティリティ
 * harfbuzzjs (WebAssembly) を使用
 */

let hbSubsetWasm: WebAssembly.Instance | null = null;
let heapu8: Uint8Array | null = null;

// WASMを遅延ロード
async function loadHarfbuzzSubset(): Promise<{
  wasm: WebAssembly.Instance['exports'];
  heapu8: Uint8Array;
}> {
  if (hbSubsetWasm && heapu8) {
    return { wasm: hbSubsetWasm.exports, heapu8 };
  }

  // harfbuzzjsのhb-subset.wasmをfetch（publicフォルダから）
  const wasmUrl = './hb-subset.wasm';

  const response = await fetch(wasmUrl);
  const wasmBuffer = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(wasmBuffer);

  hbSubsetWasm = instance;
  heapu8 = new Uint8Array((instance.exports.memory as WebAssembly.Memory).buffer);

  return { wasm: instance.exports, heapu8 };
}

/**
 * 指定した文字だけを含むサブセットフォントを作成
 * @param fontBuffer 元のフォントデータ (ArrayBuffer)
 * @param text 含める文字列
 * @returns サブセットされたフォントデータ (Uint8Array)
 */
export async function createFontSubset(
  fontBuffer: ArrayBuffer,
  text: string
): Promise<Uint8Array> {
  const { wasm } = await loadHarfbuzzSubset();

  const exports = wasm as {
    malloc: (size: number) => number;
    free: (ptr: number) => void;
    memory: WebAssembly.Memory;
    hb_blob_create: (data: number, length: number, mode: number, user_data: number, destroy: number) => number;
    hb_blob_destroy: (blob: number) => void;
    hb_face_create: (blob: number, index: number) => number;
    hb_face_destroy: (face: number) => void;
    hb_subset_input_create_or_fail: () => number;
    hb_subset_input_destroy: (input: number) => void;
    hb_subset_input_unicode_set: (input: number) => number;
    hb_subset_input_set: (input: number, set_type: number) => number;
    hb_set_add: (set: number, codepoint: number) => void;
    hb_set_clear: (set: number) => void;
    hb_set_invert: (set: number) => void;
    hb_subset_or_fail: (face: number, input: number) => number;
    hb_face_reference_blob: (face: number) => number;
    hb_blob_get_length: (blob: number) => number;
    hb_blob_get_data: (blob: number, length: number) => number;
  };

  // フォントデータをWASMメモリにコピー
  const fontData = new Uint8Array(fontBuffer);
  const fontPtr = exports.malloc(fontData.length);
  
  // メモリビューを更新（mallocでメモリが拡張された可能性があるため）
  const currentHeap = new Uint8Array((exports.memory as WebAssembly.Memory).buffer);
  currentHeap.set(fontData, fontPtr);

  try {
    // blobを作成
    const blob = exports.hb_blob_create(fontPtr, fontData.length, 2 /* HB_MEMORY_MODE_WRITABLE */, 0, 0);
    if (!blob) throw new Error('Failed to create blob');

    // faceを作成
    const face = exports.hb_face_create(blob, 0);
    exports.hb_blob_destroy(blob);
    if (!face) throw new Error('Failed to create face');

    // サブセット入力を作成
    const input = exports.hb_subset_input_create_or_fail();
    if (!input) {
      exports.hb_face_destroy(face);
      throw new Error('Failed to create subset input');
    }

    // レイアウト機能を保持
    const layoutFeatures = exports.hb_subset_input_set(input, 6 /* HB_SUBSET_SETS_LAYOUT_FEATURE_TAG */);
    exports.hb_set_clear(layoutFeatures);
    exports.hb_set_invert(layoutFeatures);

    // Unicodeコードポイントを追加
    const unicodeSet = exports.hb_subset_input_unicode_set(input);
    for (const char of text) {
      const codePoint = char.codePointAt(0);
      if (codePoint !== undefined) {
        exports.hb_set_add(unicodeSet, codePoint);
      }
    }

    // サブセットを実行
    const subsetFace = exports.hb_subset_or_fail(face, input);
    exports.hb_subset_input_destroy(input);
    exports.hb_face_destroy(face);

    if (!subsetFace) {
      throw new Error('Subset failed');
    }

    // 結果を取得
    const resultBlob = exports.hb_face_reference_blob(subsetFace);
    const resultLength = exports.hb_blob_get_length(resultBlob);
    const resultPtr = exports.hb_blob_get_data(resultBlob, 0);

    // メモリビューを再取得
    const finalHeap = new Uint8Array((exports.memory as WebAssembly.Memory).buffer);
    const result = new Uint8Array(resultLength);
    result.set(finalHeap.slice(resultPtr, resultPtr + resultLength));

    exports.hb_blob_destroy(resultBlob);
    exports.hb_face_destroy(subsetFace);

    return result;
  } finally {
    exports.free(fontPtr);
  }
}

