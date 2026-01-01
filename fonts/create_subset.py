#!/usr/bin/env python3
"""
JIS第一・第二水準漢字を含むフォントサブセットを作成
"""

# 基本ASCII + 半角カナ
basic = ''.join(chr(i) for i in range(0x20, 0x7F))  # ASCII
halfwidth_kana = ''.join(chr(i) for i in range(0xFF61, 0xFFA0))  # 半角カナ

# ひらがな（U+3040-U+309F）
hiragana = ''.join(chr(i) for i in range(0x3040, 0x30A0))

# カタカナ（U+30A0-U+30FF）
katakana = ''.join(chr(i) for i in range(0x30A0, 0x3100))

# カタカナ拡張（U+31F0-U+31FF）
katakana_ext = ''.join(chr(i) for i in range(0x31F0, 0x3200))

# 全角英数・記号（U+FF00-U+FF60）
fullwidth = ''.join(chr(i) for i in range(0xFF00, 0xFF61))

# CJK記号・句読点（U+3000-U+303F）
cjk_symbols = ''.join(chr(i) for i in range(0x3000, 0x3040))

# 一般句読点（U+2000-U+206F）
general_punct = ''.join(chr(i) for i in range(0x2000, 0x2070))

# 囲み英数字（U+2460-U+24FF）- ①②③など
enclosed_alphanum = ''.join(chr(i) for i in range(0x2460, 0x2500))

# 矢印（U+2190-U+21FF）
arrows = ''.join(chr(i) for i in range(0x2190, 0x2200))

# 数学記号（U+2200-U+22FF）
math = ''.join(chr(i) for i in range(0x2200, 0x2300))

# 罫線素片（U+2500-U+257F）
box_drawing = ''.join(chr(i) for i in range(0x2500, 0x2580))

# CJK統合漢字（U+4E00-U+9FFF）- 約21,000字、JIS第一・第二水準を含む
# 全部入れると大きすぎるので、JIS X 0208の範囲に近い主要な漢字を含める
# JIS第一水準（2,965字）+ JIS第二水準（3,390字）= 約6,355字
# ここでは一般的に使用される漢字範囲を含める

# JIS X 0208 の漢字（第一・第二水準）をUnicode順に列挙
# 参考: https://www.unicode.org/charts/PDF/U4E00.pdf
# 実用的には U+4E00-U+9FFF の中で日本語で使われる主要部分

# 常用漢字2136字 + JIS第一・第二水準を全部含めるため、
# CJK Unified Ideographs の日本語で使う範囲を含める
cjk_unified = ''.join(chr(i) for i in range(0x4E00, 0x9FD0))  # 約21,000字

# CJK互換漢字（U+F900-U+FAFF）- 一部の異体字
cjk_compat = ''.join(chr(i) for i in range(0xF900, 0xFB00))

# 追加の記号
extra_symbols = '※●○◎◇◆□■△▲▽▼→←↑↓↔⇒⇔∀∃∈∋∩∪⊂⊃⊆⊇≠≡≦≧∞∝∂∇√∫∬∮≪≫±×÷°′″℃％‰㎜㎝㎞㎡㎥㏄㌔㌢㍍㌧㌻㌶〒☆★♪♭♯♀♂'

# 全部結合
all_chars = (
    basic +
    halfwidth_kana +
    hiragana +
    katakana +
    katakana_ext +
    fullwidth +
    cjk_symbols +
    general_punct +
    enclosed_alphanum +
    arrows +
    math +
    box_drawing +
    cjk_unified +
    cjk_compat +
    extra_symbols
)

# 重複を除去してソート
unique_chars = ''.join(sorted(set(all_chars)))

# ファイルに書き出し
with open('subset_chars_full.txt', 'w', encoding='utf-8') as f:
    f.write(unique_chars)

print(f"Total unique characters: {len(unique_chars)}")
print(f"Saved to subset_chars_full.txt")
