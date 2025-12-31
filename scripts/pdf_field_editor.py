"""
PDF空欄フィールドエディター
Document AI OCRで検出した座標に基づいて、PDFの空欄フィールドを編集するGUIツール
"""

import json
import re
import sys
import tempfile
from pathlib import Path
from io import BytesIO

from PyQt6.QtWidgets import (
    QApplication,
    QMainWindow,
    QWidget,
    QVBoxLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QComboBox,
    QScrollArea,
    QGroupBox,
    QFormLayout,
    QMessageBox,
    QFileDialog,
    QSlider,
    QSpinBox,
)
from PyQt6.QtGui import QPixmap, QPainter, QPen, QColor, QFont, QImage
from PyQt6.QtCore import Qt

from google.oauth2.credentials import Credentials
from google.auth.transport.requests import Request
import googleapiclient.discovery

# PyMuPDF for PDF handling
try:
    import fitz  # PyMuPDF
except ImportError:
    print("PyMuPDFが必要です: pip install pymupdf")
    sys.exit(1)

# 設定
PROJECT_ROOT = Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator")
TOKEN_PATH = PROJECT_ROOT / "gmail_token.json"
COORDINATES_PATH = Path(r"H:\マイドライブ\〇市道 南千反畑町第１号線舗装補修工事\５施工体制\blank_field_coordinates.json")

SCOPES = [
    'https://www.googleapis.com/auth/drive',
    'https://www.googleapis.com/auth/spreadsheets',
    'https://mail.google.com/',
]


def get_drive_service():
    """Drive APIサービスを取得"""
    creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)
    if creds and creds.refresh_token:
        try:
            creds.refresh(Request())
        except Exception as e:
            print(f"トークンリフレッシュエラー: {e}")
    return googleapiclient.discovery.build('drive', 'v3', credentials=creds)


def extract_file_id(url):
    """URLからファイルIDを抽出"""
    patterns = [
        r'/file/d/([a-zA-Z0-9-_]+)',
        r'/d/([a-zA-Z0-9-_]+)',
        r'id=([a-zA-Z0-9-_]+)',
    ]
    for pattern in patterns:
        match = re.search(pattern, url)
        if match:
            return match.group(1)
    return None


def download_pdf(drive_service, file_id):
    """PDFをダウンロード"""
    request = drive_service.files().get_media(fileId=file_id)
    return request.execute()


class PDFFieldEditor(QMainWindow):
    """PDF空欄フィールドエディター"""

    def __init__(self):
        super().__init__()
        self.setWindowTitle("施工体制書類 空欄フィールドエディター")
        self.setMinimumSize(1200, 800)

        self.coordinates_data = []
        self.current_doc_index = 0
        self.pdf_content = None
        self.page_pixmap = None
        self.drive_service = None

        # 座標オフセット（微調整用）
        self.dest_offset_x = 0
        self.dest_offset_y = 0
        self.date_offset_x = 0
        self.date_offset_y = 0

        self.init_ui()
        self.load_coordinates()

    def init_ui(self):
        """UIを初期化"""
        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        main_layout = QHBoxLayout(central_widget)

        # 左側: PDFプレビュー
        preview_layout = QVBoxLayout()

        # ドキュメント選択
        doc_select_layout = QHBoxLayout()
        doc_select_layout.addWidget(QLabel("書類:"))
        self.doc_combo = QComboBox()
        self.doc_combo.setMinimumWidth(300)
        self.doc_combo.currentIndexChanged.connect(self.on_doc_changed)
        doc_select_layout.addWidget(self.doc_combo)
        doc_select_layout.addStretch()
        preview_layout.addLayout(doc_select_layout)

        # PDFプレビュー
        self.scroll_area = QScrollArea()
        self.preview_label = QLabel("PDFを読み込み中...")
        self.preview_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.preview_label.setMinimumSize(800, 600)
        self.preview_label.setStyleSheet("background-color: #e0e0e0;")
        self.scroll_area.setWidget(self.preview_label)
        self.scroll_area.setWidgetResizable(True)
        preview_layout.addWidget(self.scroll_area)

        main_layout.addLayout(preview_layout, 3)

        # 右側: 入力フォーム
        form_widget = QWidget()
        form_layout = QVBoxLayout(form_widget)

        # 書類情報
        info_group = QGroupBox("書類情報")
        info_layout = QFormLayout()
        self.contractor_label = QLabel("-")
        self.doc_type_label = QLabel("-")
        info_layout.addRow("業者:", self.contractor_label)
        info_layout.addRow("書類:", self.doc_type_label)
        info_group.setLayout(info_layout)
        form_layout.addWidget(info_group)

        # 提出先入力
        dest_group = QGroupBox("提出先（赤枠）")
        dest_layout = QVBoxLayout()

        # テキスト入力
        dest_text_layout = QHBoxLayout()
        dest_text_layout.addWidget(QLabel("テキスト:"))
        self.dest_input = QLineEdit()
        self.dest_input.setPlaceholderText("例: サンプル建設株式会社")
        self.dest_input.textChanged.connect(self.update_preview)
        dest_text_layout.addWidget(self.dest_input)
        dest_layout.addLayout(dest_text_layout)

        # X座標調整
        dest_x_layout = QHBoxLayout()
        dest_x_layout.addWidget(QLabel("X調整:"))
        self.dest_x_spin = QSpinBox()
        self.dest_x_spin.setRange(-200, 200)
        self.dest_x_spin.setValue(0)
        self.dest_x_spin.valueChanged.connect(self.on_dest_offset_changed)
        dest_x_layout.addWidget(self.dest_x_spin)
        self.dest_x_slider = QSlider(Qt.Orientation.Horizontal)
        self.dest_x_slider.setRange(-200, 200)
        self.dest_x_slider.setValue(0)
        self.dest_x_slider.valueChanged.connect(self.dest_x_spin.setValue)
        self.dest_x_spin.valueChanged.connect(self.dest_x_slider.setValue)
        dest_x_layout.addWidget(self.dest_x_slider)
        dest_layout.addLayout(dest_x_layout)

        # Y座標調整
        dest_y_layout = QHBoxLayout()
        dest_y_layout.addWidget(QLabel("Y調整:"))
        self.dest_y_spin = QSpinBox()
        self.dest_y_spin.setRange(-200, 200)
        self.dest_y_spin.setValue(0)
        self.dest_y_spin.valueChanged.connect(self.on_dest_offset_changed)
        dest_y_layout.addWidget(self.dest_y_spin)
        self.dest_y_slider = QSlider(Qt.Orientation.Horizontal)
        self.dest_y_slider.setRange(-200, 200)
        self.dest_y_slider.setValue(0)
        self.dest_y_slider.valueChanged.connect(self.dest_y_spin.setValue)
        self.dest_y_spin.valueChanged.connect(self.dest_y_slider.setValue)
        dest_y_layout.addWidget(self.dest_y_slider)
        dest_layout.addLayout(dest_y_layout)

        dest_group.setLayout(dest_layout)
        form_layout.addWidget(dest_group)

        # 日付入力
        date_group = QGroupBox("日付（青枠）")
        date_layout = QVBoxLayout()

        # 日付テキスト入力
        self.year_input = QLineEdit()
        self.year_input.setPlaceholderText("7")
        self.year_input.setMaximumWidth(60)
        self.year_input.textChanged.connect(self.update_preview)
        self.month_input = QLineEdit()
        self.month_input.setPlaceholderText("1")
        self.month_input.setMaximumWidth(60)
        self.month_input.textChanged.connect(self.update_preview)
        self.day_input = QLineEdit()
        self.day_input.setPlaceholderText("10")
        self.day_input.setMaximumWidth(60)
        self.day_input.textChanged.connect(self.update_preview)

        date_h_layout = QHBoxLayout()
        date_h_layout.addWidget(QLabel("令和"))
        date_h_layout.addWidget(self.year_input)
        date_h_layout.addWidget(QLabel("年"))
        date_h_layout.addWidget(self.month_input)
        date_h_layout.addWidget(QLabel("月"))
        date_h_layout.addWidget(self.day_input)
        date_h_layout.addWidget(QLabel("日"))
        date_h_layout.addStretch()
        date_layout.addLayout(date_h_layout)

        # X座標調整
        date_x_layout = QHBoxLayout()
        date_x_layout.addWidget(QLabel("X調整:"))
        self.date_x_spin = QSpinBox()
        self.date_x_spin.setRange(-200, 200)
        self.date_x_spin.setValue(0)
        self.date_x_spin.valueChanged.connect(self.on_date_offset_changed)
        date_x_layout.addWidget(self.date_x_spin)
        self.date_x_slider = QSlider(Qt.Orientation.Horizontal)
        self.date_x_slider.setRange(-200, 200)
        self.date_x_slider.setValue(0)
        self.date_x_slider.valueChanged.connect(self.date_x_spin.setValue)
        self.date_x_spin.valueChanged.connect(self.date_x_slider.setValue)
        date_x_layout.addWidget(self.date_x_slider)
        date_layout.addLayout(date_x_layout)

        # Y座標調整
        date_y_layout = QHBoxLayout()
        date_y_layout.addWidget(QLabel("Y調整:"))
        self.date_y_spin = QSpinBox()
        self.date_y_spin.setRange(-200, 200)
        self.date_y_spin.setValue(0)
        self.date_y_spin.valueChanged.connect(self.on_date_offset_changed)
        date_y_layout.addWidget(self.date_y_spin)
        self.date_y_slider = QSlider(Qt.Orientation.Horizontal)
        self.date_y_slider.setRange(-200, 200)
        self.date_y_slider.setValue(0)
        self.date_y_slider.valueChanged.connect(self.date_y_spin.setValue)
        self.date_y_spin.valueChanged.connect(self.date_y_slider.setValue)
        date_y_layout.addWidget(self.date_y_slider)
        date_layout.addLayout(date_y_layout)

        date_group.setLayout(date_layout)
        form_layout.addWidget(date_group)

        # ボタン
        button_layout = QHBoxLayout()
        self.save_btn = QPushButton("PDFに書き込んで保存")
        self.save_btn.clicked.connect(self.save_pdf)
        self.save_btn.setStyleSheet("background-color: #4CAF50; color: white; padding: 10px;")
        button_layout.addWidget(self.save_btn)
        form_layout.addLayout(button_layout)

        form_layout.addStretch()
        main_layout.addWidget(form_widget, 1)

    def load_coordinates(self):
        """座標データを読み込み"""
        try:
            with open(COORDINATES_PATH, 'r', encoding='utf-8') as f:
                self.coordinates_data = json.load(f)

            # コンボボックスに追加
            for doc in self.coordinates_data:
                contractor = doc.get('contractor', '不明')
                doc_type = doc.get('doc_type', '不明')
                self.doc_combo.addItem(f"{contractor} - {doc_type}")

            if self.coordinates_data:
                self.on_doc_changed(0)

        except Exception as e:
            QMessageBox.critical(self, "エラー", f"座標データの読み込みに失敗: {e}")

    def on_doc_changed(self, index):
        """ドキュメント選択が変更された"""
        if index < 0 or index >= len(self.coordinates_data):
            return

        self.current_doc_index = index
        doc = self.coordinates_data[index]

        # 情報を表示
        self.contractor_label.setText(doc.get('contractor', '-'))
        self.doc_type_label.setText(doc.get('doc_type', '-'))

        # 入力フィールドをクリア
        self.dest_input.clear()
        self.year_input.clear()
        self.month_input.clear()
        self.day_input.clear()

        # オフセットをリセット
        self.dest_x_spin.setValue(0)
        self.dest_y_spin.setValue(0)
        self.date_x_spin.setValue(0)
        self.date_y_spin.setValue(0)

        # PDFを読み込み
        self.load_pdf(doc.get('url', ''))

    def on_dest_offset_changed(self):
        """提出先オフセットが変更された"""
        self.dest_offset_x = self.dest_x_spin.value()
        self.dest_offset_y = self.dest_y_spin.value()
        self.update_preview()

    def on_date_offset_changed(self):
        """日付オフセットが変更された"""
        self.date_offset_x = self.date_x_spin.value()
        self.date_offset_y = self.date_y_spin.value()
        self.update_preview()

    def load_pdf(self, url):
        """Google DriveからPDFを読み込み"""
        self.preview_label.setText("PDFを読み込み中...")
        QApplication.processEvents()

        try:
            if not self.drive_service:
                self.drive_service = get_drive_service()

            file_id = extract_file_id(url)
            if not file_id:
                self.preview_label.setText("URLからファイルIDを取得できません")
                return

            self.pdf_content = download_pdf(self.drive_service, file_id)
            self.render_pdf_page()

        except Exception as e:
            self.preview_label.setText(f"PDF読み込みエラー: {e}")

    def render_pdf_page(self, page_num=0):
        """PDFページを画像として描画"""
        if not self.pdf_content:
            return

        try:
            # PyMuPDFでPDFを開く
            doc = fitz.open(stream=self.pdf_content, filetype="pdf")
            page = doc.load_page(page_num)

            # 高解像度でレンダリング (2倍スケール)
            mat = fitz.Matrix(2.0, 2.0)
            pix = page.get_pixmap(matrix=mat)

            # QImageに変換
            img = QImage(pix.samples, pix.width, pix.height, pix.stride, QImage.Format.Format_RGB888)
            self.page_pixmap = QPixmap.fromImage(img)

            doc.close()

            self.update_preview()

        except Exception as e:
            self.preview_label.setText(f"PDF描画エラー: {e}")

    def update_preview(self):
        """プレビューを更新（入力テキストを反映）"""
        if not self.page_pixmap:
            return

        # コピーを作成して描画
        display_pixmap = self.page_pixmap.copy()
        painter = QPainter(display_pixmap)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        doc = self.coordinates_data[self.current_doc_index]
        coords = doc.get('coordinates', {})

        # スケール（2倍でレンダリングしているため）
        scale = 2.0

        # 提出先を描画
        dest_coords = coords.get('destination')
        if dest_coords and dest_coords.get('fill_position'):
            fill_pos = dest_coords['fill_position']
            marker_coords = dest_coords.get('marker_coords', {})
            page_size = marker_coords.get('page_size', {'width': 1681, 'height': 2378})

            # オフセットを適用
            x = fill_pos['x'] * page_size['width'] * scale + self.dest_offset_x
            y = fill_pos['y'] * page_size['height'] * scale + self.dest_offset_y
            box_width = int(fill_pos.get('suggested_width', 0.18) * page_size['width'] * scale)
            box_height = 50

            # 空欄領域を半透明の赤でハイライト（常に表示）
            painter.fillRect(int(x), int(y - 10), box_width, box_height, QColor(255, 0, 0, 60))
            pen = QPen(QColor(255, 0, 0), 3)
            painter.setPen(pen)
            painter.drawRect(int(x), int(y - 10), box_width, box_height)

            # ラベル表示
            label_font = QFont("MS Gothic", 14)
            painter.setFont(label_font)
            painter.setPen(QPen(QColor(255, 0, 0)))
            painter.drawText(int(x), int(y - 15), "▼ 提出先をここに入力")

            # 入力テキストを描画
            dest_text = self.dest_input.text()
            text_font = QFont("MS Gothic", 24)
            text_font.setBold(True)
            painter.setFont(text_font)
            if dest_text:
                painter.setPen(QPen(QColor(0, 0, 0)))
                painter.drawText(int(x + 10), int(y + 25), dest_text)
            else:
                # プレースホルダー
                painter.setPen(QPen(QColor(180, 180, 180)))
                painter.drawText(int(x + 10), int(y + 25), "（未入力）")

        # 日付を描画
        date_coords = coords.get('date')
        if date_coords and date_coords.get('fill_positions'):
            fill_positions = date_coords['fill_positions']
            marker_coords = date_coords.get('marker_coords', {})
            page_size = marker_coords.get('page_size', {'width': 1681, 'height': 2378})

            # 年
            year_pos = fill_positions.get('year')
            if year_pos:
                x = year_pos['x'] * page_size['width'] * scale + self.date_offset_x
                y = year_pos['y'] * page_size['height'] * scale + self.date_offset_y
                box_width = 60
                box_height = 50

                # ハイライト（常に表示）
                painter.fillRect(int(x), int(y - 10), box_width, box_height, QColor(0, 0, 255, 60))
                pen = QPen(QColor(0, 0, 255), 3)
                painter.setPen(pen)
                painter.drawRect(int(x), int(y - 10), box_width, box_height)

                # ラベル
                label_font = QFont("MS Gothic", 12)
                painter.setFont(label_font)
                painter.setPen(QPen(QColor(0, 0, 255)))
                painter.drawText(int(x), int(y - 15), "年")

                # 入力テキスト
                text_font = QFont("MS Gothic", 24)
                text_font.setBold(True)
                painter.setFont(text_font)
                year_text = self.year_input.text()
                if year_text:
                    painter.setPen(QPen(QColor(0, 0, 0)))
                    painter.drawText(int(x + 10), int(y + 25), year_text)
                else:
                    painter.setPen(QPen(QColor(180, 180, 180)))
                    painter.drawText(int(x + 10), int(y + 25), "○")

            # 月
            month_pos = fill_positions.get('month')
            if month_pos and year_pos:
                x = month_pos['x'] * page_size['width'] * scale + self.date_offset_x
                y_raw = month_pos['y'] * page_size['height'] * scale

                # 注：month_posのyが異常値の場合は年と同じ行に
                if y_raw > page_size['height'] * scale:
                    y = year_pos['y'] * page_size['height'] * scale + self.date_offset_y
                else:
                    y = y_raw + self.date_offset_y

                box_width = 50
                box_height = 50

                painter.fillRect(int(x), int(y - 10), box_width, box_height, QColor(0, 0, 255, 60))
                pen = QPen(QColor(0, 0, 255), 3)
                painter.setPen(pen)
                painter.drawRect(int(x), int(y - 10), box_width, box_height)

                # ラベル
                label_font = QFont("MS Gothic", 12)
                painter.setFont(label_font)
                painter.setPen(QPen(QColor(0, 0, 255)))
                painter.drawText(int(x), int(y - 15), "月")

                text_font = QFont("MS Gothic", 24)
                text_font.setBold(True)
                painter.setFont(text_font)
                month_text = self.month_input.text()
                if month_text:
                    painter.setPen(QPen(QColor(0, 0, 0)))
                    painter.drawText(int(x + 10), int(y + 25), month_text)
                else:
                    painter.setPen(QPen(QColor(180, 180, 180)))
                    painter.drawText(int(x + 10), int(y + 25), "○")

            # 日
            day_pos = fill_positions.get('day')
            if day_pos and year_pos:
                x = day_pos['x'] * page_size['width'] * scale + self.date_offset_x
                y = day_pos['y'] * page_size['height'] * scale + self.date_offset_y
                box_width = 50
                box_height = 50

                painter.fillRect(int(x), int(y - 10), box_width, box_height, QColor(0, 0, 255, 60))
                pen = QPen(QColor(0, 0, 255), 3)
                painter.setPen(pen)
                painter.drawRect(int(x), int(y - 10), box_width, box_height)

                # ラベル
                label_font = QFont("MS Gothic", 12)
                painter.setFont(label_font)
                painter.setPen(QPen(QColor(0, 0, 255)))
                painter.drawText(int(x), int(y - 15), "日")

                text_font = QFont("MS Gothic", 24)
                text_font.setBold(True)
                painter.setFont(text_font)
                day_text = self.day_input.text()
                if day_text:
                    painter.setPen(QPen(QColor(0, 0, 0)))
                    painter.drawText(int(x + 10), int(y + 25), day_text)
                else:
                    painter.setPen(QPen(QColor(180, 180, 180)))
                    painter.drawText(int(x + 10), int(y + 25), "○")

        painter.end()

        # 表示サイズに縮小
        scaled = display_pixmap.scaled(
            self.scroll_area.width() - 30,
            self.scroll_area.height() - 30,
            Qt.AspectRatioMode.KeepAspectRatio,
            Qt.TransformationMode.SmoothTransformation
        )
        self.preview_label.setPixmap(scaled)

    def save_pdf(self):
        """入力内容をPDFに書き込んで保存"""
        if not self.pdf_content:
            QMessageBox.warning(self, "警告", "PDFが読み込まれていません")
            return

        doc_data = self.coordinates_data[self.current_doc_index]
        coords = doc_data.get('coordinates', {})

        try:
            # PyMuPDFでPDFを開く
            pdf_doc = fitz.open(stream=self.pdf_content, filetype="pdf")
            page = pdf_doc.load_page(0)

            # フォント設定
            fontname = "japan"  # 日本語フォント

            # 提出先を書き込み
            dest_text = self.dest_input.text()
            dest_coords = coords.get('destination')
            if dest_text and dest_coords and dest_coords.get('fill_position'):
                fill_pos = dest_coords['fill_position']
                marker_coords = dest_coords.get('marker_coords', {})
                page_size = marker_coords.get('page_size', {'width': 1681, 'height': 2378})

                # PDF座標系に変換（オフセットを適用、スケール2.0で描画しているので半分に）
                x = fill_pos['x'] * page_size['width'] + self.dest_offset_x / 2
                y = fill_pos['y'] * page_size['height'] + self.dest_offset_y / 2

                # テキストを挿入
                page.insert_text(
                    fitz.Point(x + 5, y + 15),
                    dest_text,
                    fontsize=12,
                    fontname=fontname,
                    color=(0, 0, 0)
                )

            # 日付を書き込み
            date_coords = coords.get('date')
            if date_coords and date_coords.get('fill_positions'):
                fill_positions = date_coords['fill_positions']
                marker_coords = date_coords.get('marker_coords', {})
                page_size = marker_coords.get('page_size', {'width': 1681, 'height': 2378})

                # 年
                year_text = self.year_input.text()
                year_pos = fill_positions.get('year')
                if year_text and year_pos:
                    x = year_pos['x'] * page_size['width'] + self.date_offset_x / 2
                    y = year_pos['y'] * page_size['height'] + self.date_offset_y / 2
                    page.insert_text(fitz.Point(x + 5, y + 15), year_text, fontsize=12, fontname=fontname)

                # 月
                month_text = self.month_input.text()
                month_pos = fill_positions.get('month')
                if month_text and month_pos:
                    x = month_pos['x'] * page_size['width'] + self.date_offset_x / 2
                    # y座標が異常な場合は年と同じ行に
                    y = month_pos['y'] * page_size['height']
                    if y > page_size['height']:
                        y = year_pos['y'] * page_size['height'] if year_pos else 400
                    y = y + self.date_offset_y / 2
                    page.insert_text(fitz.Point(x + 5, y + 15), month_text, fontsize=12, fontname=fontname)

                # 日
                day_text = self.day_input.text()
                day_pos = fill_positions.get('day')
                if day_text and day_pos:
                    x = day_pos['x'] * page_size['width'] + self.date_offset_x / 2
                    y = day_pos['y'] * page_size['height'] + self.date_offset_y / 2
                    page.insert_text(fitz.Point(x + 5, y + 15), day_text, fontsize=12, fontname=fontname)

            # 保存先を選択
            contractor = doc_data.get('contractor', '業者')
            doc_type = doc_data.get('doc_type', '書類')
            default_name = f"{contractor}_{doc_type}_記入済み.pdf"

            save_path, _ = QFileDialog.getSaveFileName(
                self,
                "PDFを保存",
                str(Path.home() / "Downloads" / default_name),
                "PDF Files (*.pdf)"
            )

            if save_path:
                pdf_doc.save(save_path)
                pdf_doc.close()
                QMessageBox.information(self, "完了", f"保存しました:\n{save_path}")
            else:
                pdf_doc.close()

        except Exception as e:
            QMessageBox.critical(self, "エラー", f"PDF保存エラー: {e}")

    def resizeEvent(self, event):
        """ウィンドウリサイズ時にプレビューを更新"""
        super().resizeEvent(event)
        self.update_preview()


def main():
    app = QApplication(sys.argv)
    app.setStyle("Fusion")

    window = PDFFieldEditor()
    window.show()

    sys.exit(app.exec())


if __name__ == '__main__':
    main()
