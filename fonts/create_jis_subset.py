#!/usr/bin/env python3
"""
JIS X 0208 第一・第二水準漢字 + 基本文字セットでフォントサブセットを作成
"""
import codecs

def get_jis_x_0208_chars():
    """JIS X 0208の全文字を取得（第一・第二水準漢字を含む）"""
    chars = set()

    # JIS X 0208 は 区点コードで定義
    # 第一水準漢字: 16区〜47区
    # 第二水準漢字: 48区〜84区
    # 非漢字: 1区〜8区（記号、英数字、ひらがな、カタカナなど）

    for ku in range(1, 95):  # 1区〜94区
        for ten in range(1, 95):  # 1点〜94点
            # 区点コードをJISコードに変換
            jis_code = ((ku + 0x20) << 8) | (ten + 0x20)
            try:
                # JISコードをバイト列に変換
                jis_bytes = bytes([ku + 0x20, ten + 0x20])
                # iso-2022-jpでデコード（ESCシーケンス付き）
                jis_with_esc = b'\x1b$B' + jis_bytes + b'\x1b(B'
                char = jis_with_esc.decode('iso-2022-jp')
                if char and char.strip():
                    chars.add(char)
            except UnicodeDecodeError:
                pass

    return chars

# JIS X 0208 文字を取得
jis_chars = get_jis_x_0208_chars()

# 基本ASCII（半角英数記号）
basic_ascii = ''.join(chr(i) for i in range(0x20, 0x7F))

# 半角カナ
halfwidth_kana = ''.join(chr(i) for i in range(0xFF61, 0xFFA0))

# 全角英数（念のため）
fullwidth = ''.join(chr(i) for i in range(0xFF01, 0xFF5F))

# 追加の記号（JISに含まれないかもしれないもの）
extra_symbols = '〒☆★♪♀♂㎜㎝㎞㎡㎥㏄㌔㌢㍍㌧㌻㌶'

# 全部結合
all_chars = set(basic_ascii) | set(halfwidth_kana) | set(fullwidth) | jis_chars | set(extra_symbols)

# ソートして文字列に
unique_chars = ''.join(sorted(all_chars))

# ファイルに書き出し
with open('subset_chars_jis.txt', 'w', encoding='utf-8') as f:
    f.write(unique_chars)

print(f"JIS X 0208 characters: {len(jis_chars)}")
print(f"Total unique characters: {len(unique_chars)}")
print(f"Saved to subset_chars_jis.txt")

# 含まれる文字の種類を確認
kanji_count = sum(1 for c in unique_chars if '\u4e00' <= c <= '\u9fff')
hiragana_count = sum(1 for c in unique_chars if '\u3040' <= c <= '\u309f')
katakana_count = sum(1 for c in unique_chars if '\u30a0' <= c <= '\u30ff')
print(f"  - Kanji: {kanji_count}")
print(f"  - Hiragana: {hiragana_count}")
print(f"  - Katakana: {katakana_count}")
