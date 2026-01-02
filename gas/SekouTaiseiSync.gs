/**
 * 施工体制台帳メーカー スプレッドシート同期 GAS
 *
 * 使い方:
 * 1. Google スプレッドシートを新規作成
 * 2. 拡張機能 > Apps Script を開く
 * 3. このコードを貼り付けて保存
 * 4. デプロイ > 新しいデプロイ > ウェブアプリ を選択
 * 5. 「アクセスできるユーザー」を「全員」に設定
 * 6. デプロイしてURLを取得
 * 7. アプリの設定画面でそのURLを入力
 */

const CONFIG = {
  DATA_SHEET: 'ProjectData',
  HISTORY_SHEET: 'History',
  SETTINGS_SHEET: '_Settings'
};

// POSTリクエスト処理（データ保存）
function doPost(e) {
  try {
    // CORSヘッダー付きレスポンス用
    const data = JSON.parse(e.postData.contents);

    if (data.action === 'save') {
      const result = saveProject(data.project);
      return jsonResponse(result);
    }

    // PDF アップロード（上書き or 別名保存）
    if (data.action === 'uploadPdf') {
      const result = uploadPdfToDrive(data);
      return jsonResponse(result);
    }

    // 設定保存（暗号化APIキー等）
    if (data.action === 'saveSettings') {
      const result = saveSettings(data.settings);
      return jsonResponse(result);
    }

    return jsonResponse({ error: 'Unknown action' });
  } catch (err) {
    return jsonResponse({ error: err.message });
  }
}

// OPTIONSリクエスト処理（CORSプリフライト対応）
function doOptions(e) {
  return ContentService.createTextOutput('')
    .setMimeType(ContentService.MimeType.TEXT);
}

// GETリクエスト処理（データ取得）
function doGet(e) {
  try {
    const action = e.parameter.action;

    // PDF取得アクション
    if (action === 'fetchPdf') {
      const fileId = e.parameter.fileId;
      if (!fileId) {
        return jsonResponse({ error: 'fileId is required' });
      }
      const result = fetchPdfAsBase64(fileId);
      return jsonResponse(result);
    }

    // ファイル情報取得（フォルダID等）
    if (action === 'getFileInfo') {
      const fileId = e.parameter.fileId;
      if (!fileId) {
        return jsonResponse({ error: 'fileId is required' });
      }
      const result = getFileInfo(fileId);
      return jsonResponse(result);
    }

    // 設定のみ取得
    if (action === 'loadSettings') {
      const settings = loadSettings();
      return jsonResponse(settings);
    }

    // デフォルト: プロジェクトデータ取得（設定も含む）
    const data = loadProject();
    const settings = loadSettings();
    data.settings = settings;
    return jsonResponse(data);
  } catch (err) {
    return jsonResponse({ error: err.message });
  }
}

// Google DriveからPDFを取得してBase64で返す
function fetchPdfAsBase64(fileId) {
  try {
    const file = DriveApp.getFileById(fileId);
    const blob = file.getBlob();
    const mimeType = blob.getContentType();

    // PDFでない場合はPDFに変換を試みる
    let pdfBlob;
    if (mimeType === 'application/pdf') {
      pdfBlob = blob;
    } else if (mimeType.includes('google-apps.document') ||
               mimeType.includes('google-apps.spreadsheet') ||
               mimeType.includes('google-apps.presentation')) {
      // Google Docs/Sheets/SlidesはPDFエクスポート
      const exportUrl = 'https://www.googleapis.com/drive/v3/files/' + fileId + '/export?mimeType=application/pdf';
      const response = UrlFetchApp.fetch(exportUrl, {
        headers: { Authorization: 'Bearer ' + ScriptApp.getOAuthToken() }
      });
      pdfBlob = response.getBlob();
    } else {
      return { error: 'Unsupported file type: ' + mimeType };
    }

    const base64 = Utilities.base64Encode(pdfBlob.getBytes());
    return {
      success: true,
      fileName: file.getName(),
      mimeType: 'application/pdf',
      base64: base64
    };
  } catch (err) {
    return { error: 'Failed to fetch PDF: ' + err.message };
  }
}

// ファイル情報取得（フォルダID、ファイル名）
function getFileInfo(fileId) {
  try {
    const file = DriveApp.getFileById(fileId);
    const parents = file.getParents();
    let folderId = null;
    if (parents.hasNext()) {
      folderId = parents.next().getId();
    }
    return {
      success: true,
      fileId: fileId,
      fileName: file.getName(),
      folderId: folderId
    };
  } catch (err) {
    return { error: 'Failed to get file info: ' + err.message };
  }
}

// PDFをGoogle Driveにアップロード
function uploadPdfToDrive(data) {
  try {
    const base64 = data.base64;
    const originalFileId = data.originalFileId;
    const newFileName = data.newFileName;
    const overwrite = data.overwrite;

    // Base64をBlobに変換
    const decoded = Utilities.base64Decode(base64);
    const blob = Utilities.newBlob(decoded, 'application/pdf', newFileName);

    // 元ファイルのフォルダを取得
    const originalFile = DriveApp.getFileById(originalFileId);
    const parents = originalFile.getParents();
    let folder = null;
    if (parents.hasNext()) {
      folder = parents.next();
    } else {
      folder = DriveApp.getRootFolder();
    }

    let savedFile;
    if (overwrite) {
      // 上書き: 元ファイルの内容を置き換え
      // Drive APIでは直接上書きできないため、削除→新規作成
      const originalName = originalFile.getName();
      originalFile.setTrashed(true);
      savedFile = folder.createFile(blob);
      savedFile.setName(originalName);
    } else {
      // 別名保存: 同じフォルダに新規作成
      savedFile = folder.createFile(blob);
      savedFile.setName(newFileName);
    }

    return {
      success: true,
      fileId: savedFile.getId(),
      fileName: savedFile.getName(),
      fileUrl: savedFile.getUrl()
    };
  } catch (err) {
    return { error: 'Failed to upload PDF: ' + err.message };
  }
}

// JSONレスポンス作成
function jsonResponse(data) {
  return ContentService.createTextOutput(JSON.stringify(data))
    .setMimeType(ContentService.MimeType.JSON);
}

// プロジェクトデータ保存
function saveProject(project) {
  const sheet = getOrCreateDataSheet();
  const now = new Date();
  const timestamp = Utilities.formatDate(now, 'Asia/Tokyo', 'yyyy-MM-dd HH:mm:ss');

  // JSONを整形して保存
  const jsonStr = JSON.stringify(project, null, 2);

  // A1にタイムスタンプ、A2にJSONデータ
  sheet.getRange('A1').setValue('最終更新: ' + timestamp);
  sheet.getRange('A2').setValue(jsonStr);

  // 履歴に追加
  addHistory(project.project_name, timestamp);

  return {
    success: true,
    timestamp: timestamp,
    project_name: project.project_name
  };
}

// プロジェクトデータ読み込み
function loadProject() {
  const sheet = getOrCreateDataSheet();
  const jsonStr = sheet.getRange('A2').getValue();

  if (!jsonStr) {
    return { project: null, message: 'No data' };
  }

  try {
    const project = JSON.parse(jsonStr);
    const timestamp = sheet.getRange('A1').getValue();
    return {
      project: project,
      timestamp: timestamp
    };
  } catch (err) {
    return { error: 'Invalid JSON data', message: err.message };
  }
}

// データシート取得または作成
function getOrCreateDataSheet() {
  const ss = SpreadsheetApp.getActiveSpreadsheet();
  let sheet = ss.getSheetByName(CONFIG.DATA_SHEET);

  if (!sheet) {
    sheet = ss.insertSheet(CONFIG.DATA_SHEET);
    sheet.getRange('A1').setValue('データ未保存');
    // 列幅を広げる
    sheet.setColumnWidth(1, 800);
  }

  return sheet;
}

// 履歴シート取得または作成
function getOrCreateHistorySheet() {
  const ss = SpreadsheetApp.getActiveSpreadsheet();
  let sheet = ss.getSheetByName(CONFIG.HISTORY_SHEET);

  if (!sheet) {
    sheet = ss.insertSheet(CONFIG.HISTORY_SHEET);
    sheet.getRange('A1:C1').setValues([['日時', '工事名', 'アクション']]);
    sheet.getRange('A1:C1')
      .setBackground('#4a5568')
      .setFontColor('#ffffff')
      .setFontWeight('bold');
    sheet.setFrozenRows(1);
  }

  return sheet;
}

// 履歴追加
function addHistory(projectName, timestamp) {
  const sheet = getOrCreateHistorySheet();
  sheet.insertRowAfter(1);
  sheet.getRange('A2:C2').setValues([[timestamp, projectName, '保存']]);
}

// テスト用
function testSave() {
  const testProject = {
    project_name: 'テスト工事',
    client: 'テスト市',
    period: '令和7年1月〜令和7年3月',
    contractors: []
  };

  const result = saveProject(testProject);
  console.log(result);
}

function testLoad() {
  const result = loadProject();
  console.log(result);
}

// Drive権限テスト用（初回実行で権限付与ダイアログが表示される）
function testDriveAccess() {
  try {
    // ルートフォルダにアクセスして権限を確認
    const rootFolder = DriveApp.getRootFolder();
    console.log('Drive権限OK: ルートフォルダ名 = ' + rootFolder.getName());

    // ファイル作成・削除テスト
    const testFile = rootFolder.createFile('test_permission.txt', 'テスト');
    console.log('ファイル作成OK: ' + testFile.getId());
    testFile.setTrashed(true);
    console.log('ファイル削除OK');

    return { success: true, message: 'Drive権限が正常に付与されています' };
  } catch (err) {
    console.log('Drive権限エラー: ' + err.message);
    return { success: false, error: err.message };
  }
}

// PDFアップロードテスト（ファイルIDを指定）
function testUpload(fileId) {
  if (!fileId) {
    console.log('使い方: testUpload("ファイルID")');
    return;
  }

  try {
    const file = DriveApp.getFileById(fileId);
    console.log('ファイル取得OK: ' + file.getName());

    const parents = file.getParents();
    if (parents.hasNext()) {
      console.log('親フォルダ: ' + parents.next().getName());
    }

    return { success: true, fileName: file.getName() };
  } catch (err) {
    console.log('エラー: ' + err.message);
    return { success: false, error: err.message };
  }
}

// ============================================
// 設定シート管理
// ============================================

// 設定シート取得または作成（非表示）
function getOrCreateSettingsSheet() {
  const ss = SpreadsheetApp.getActiveSpreadsheet();
  let sheet = ss.getSheetByName(CONFIG.SETTINGS_SHEET);

  if (!sheet) {
    sheet = ss.insertSheet(CONFIG.SETTINGS_SHEET);
    // ヘッダー行
    sheet.getRange('A1:B1').setValues([['キー', '値']]);
    sheet.getRange('A1:B1')
      .setBackground('#333333')
      .setFontColor('#ffffff')
      .setFontWeight('bold');
    // 列幅
    sheet.setColumnWidth(1, 200);
    sheet.setColumnWidth(2, 600);
    // シートを非表示に
    sheet.hideSheet();
  }

  return sheet;
}

// 設定保存
function saveSettings(settings) {
  try {
    const sheet = getOrCreateSettingsSheet();

    // 各設定項目を保存
    for (const key in settings) {
      const value = settings[key];
      saveSettingValue(sheet, key, value);
    }

    return { success: true };
  } catch (err) {
    return { error: 'Failed to save settings: ' + err.message };
  }
}

// 単一設定値を保存
function saveSettingValue(sheet, key, value) {
  const data = sheet.getDataRange().getValues();

  // 既存のキーを探す
  for (let i = 1; i < data.length; i++) {
    if (data[i][0] === key) {
      // 既存の値を更新
      sheet.getRange(i + 1, 2).setValue(value);
      return;
    }
  }

  // 新規追加
  const lastRow = sheet.getLastRow();
  sheet.getRange(lastRow + 1, 1, 1, 2).setValues([[key, value]]);
}

// 設定読み込み
function loadSettings() {
  try {
    const ss = SpreadsheetApp.getActiveSpreadsheet();
    const sheet = ss.getSheetByName(CONFIG.SETTINGS_SHEET);

    if (!sheet) {
      return { encryptedApiKey: null };
    }

    const data = sheet.getDataRange().getValues();
    const settings = {};

    // ヘッダー行をスキップして読み込み
    for (let i = 1; i < data.length; i++) {
      const key = data[i][0];
      const value = data[i][1];
      if (key) {
        settings[key] = value;
      }
    }

    return settings;
  } catch (err) {
    console.log('Failed to load settings: ' + err.message);
    return { encryptedApiKey: null };
  }
}

// 設定テスト
function testSettings() {
  // 保存テスト
  const result = saveSettings({
    encryptedApiKey: 'test_encrypted_key_here',
    testSetting: 'test_value'
  });
  console.log('Save result:', result);

  // 読み込みテスト
  const loaded = loadSettings();
  console.log('Loaded settings:', loaded);
}
