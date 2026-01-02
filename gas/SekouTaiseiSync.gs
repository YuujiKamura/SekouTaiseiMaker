/**
 * 施工体制台帳メーカー スプレッドシート同期 GAS
 *
 * ■ 変更履歴 (要再デプロイ)
 * ─────────────────────────────────────
 * 2026-01-02: listSheets に重要フィールド抽出機能追加
 *             → 事業所名・所長名・作成日・提出日・工事名を自動抽出
 * 2026-01-02: fetchExcelAsBase64 アクション追加
 *             → ExcelファイルをBase64で取得（フロントエンドでパース用）
 * 2026-01-02: listSheets アクション追加
 *             → スプレッドシートの全シート一覧を取得
 * 2026-01-02: fetchSpreadsheet アクション追加
 *             → 外部スプレッドシートのデータをAIチェック用に取得
 * 2026-01-02: updateDocUrl アクション追加
 *             → fileId変更時にスプレッドシートのURLを自動更新
 * 2026-01-02: getLatestFile アクション追加
 *             → フォルダ内の同名/最新PDFを自動検出
 * 2026-01-02: fetchPdfAsBase64 を Drive API 直接呼び出しに変更
 *             → Google CDNキャッシュをバイパス
 * 2026-01-02: getFileInfo に modifiedTime 追加
 *             → PDFキャッシュの有効性検証用
 * ─────────────────────────────────────
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

    // フォルダ内の最新ファイルを取得（古いfileIdから親フォルダを特定し、同名or最新のファイルを返す）
    if (action === 'getLatestFile') {
      const fileId = e.parameter.fileId;
      if (!fileId) {
        return jsonResponse({ error: 'fileId is required' });
      }
      const result = getLatestFileInFolder(fileId);
      return jsonResponse(result);
    }

    // ドキュメントURL更新（fileId変更時、GETで処理）
    if (action === 'updateDocUrl') {
      const contractorId = e.parameter.contractorId;
      const docKey = e.parameter.docKey;
      const newFileId = e.parameter.newFileId;
      if (!contractorId || !docKey || !newFileId) {
        return jsonResponse({ error: 'contractorId, docKey, newFileId are required' });
      }
      const result = updateDocUrl(contractorId, docKey, newFileId);
      return jsonResponse(result);
    }

    // スプレッドシートのシート一覧取得
    if (action === 'listSheets') {
      const spreadsheetId = e.parameter.spreadsheetId;
      if (!spreadsheetId) {
        return jsonResponse({ error: 'spreadsheetId is required' });
      }
      const result = listSpreadsheetSheets(spreadsheetId);
      return jsonResponse(result);
    }

    // ExcelファイルをBase64で取得（フロントエンドでSheetJSパース用）
    if (action === 'fetchExcelAsBase64') {
      const fileId = e.parameter.fileId;
      if (!fileId) {
        return jsonResponse({ error: 'fileId is required' });
      }
      const result = fetchExcelAsBase64(fileId);
      return jsonResponse(result);
    }

    // スプレッドシートデータ取得（AIチェック用）
    if (action === 'fetchSpreadsheet') {
      const spreadsheetId = e.parameter.spreadsheetId;
      const gid = e.parameter.gid;  // シートID（オプション）
      if (!spreadsheetId) {
        return jsonResponse({ error: 'spreadsheetId is required' });
      }
      const result = fetchSpreadsheetData(spreadsheetId, gid);
      return jsonResponse(result);
    }

    // スクリプト情報取得（UI上で更新日時表示用）
    if (action === 'getScriptInfo') {
      const result = getScriptInfo();
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
// Drive APIを直接使用してキャッシュをバイパス
function fetchPdfAsBase64(fileId) {
  try {
    const file = DriveApp.getFileById(fileId);
    const mimeType = file.getMimeType();
    const fileName = file.getName();

    let pdfBlob;
    if (mimeType === 'application/pdf') {
      // Drive API v3で直接取得（キャッシュバイパス）
      const url = 'https://www.googleapis.com/drive/v3/files/' + fileId + '?alt=media';
      const response = UrlFetchApp.fetch(url, {
        headers: { Authorization: 'Bearer ' + ScriptApp.getOAuthToken() },
        muteHttpExceptions: true
      });
      if (response.getResponseCode() !== 200) {
        return { error: 'Drive API error: ' + response.getContentText() };
      }
      pdfBlob = response.getBlob();
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
      fileName: fileName,
      mimeType: 'application/pdf',
      base64: base64,
      modifiedTime: file.getLastUpdated().toISOString()
    };
  } catch (err) {
    return { error: 'Failed to fetch PDF: ' + err.message };
  }
}

// ファイル情報取得（フォルダID、ファイル名、更新日時、MIMEタイプ）
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
      folderId: folderId,
      mimeType: file.getMimeType(),
      modifiedTime: file.getLastUpdated().toISOString()
    };
  } catch (err) {
    return { error: 'Failed to get file info: ' + err.message };
  }
}

// フォルダ内の最新ファイルを取得
// 古いfileIdから親フォルダを特定し、同名ファイルがあればそれを、なければ最新のPDFを返す
function getLatestFileInFolder(oldFileId) {
  try {
    // 古いファイルの情報を取得
    let oldFile;
    let oldFileName = null;
    let folderId = null;

    try {
      oldFile = DriveApp.getFileById(oldFileId);
      oldFileName = oldFile.getName();
      const parents = oldFile.getParents();
      if (parents.hasNext()) {
        folderId = parents.next().getId();
      }
    } catch (e) {
      // 古いファイルが見つからない場合はエラー
      return { error: 'Original file not found: ' + oldFileId };
    }

    if (!folderId) {
      // フォルダが見つからない場合は元のファイル情報を返す
      return {
        success: true,
        fileId: oldFileId,
        fileName: oldFileName,
        folderId: null,
        modifiedTime: oldFile.getLastUpdated().toISOString(),
        isLatest: true
      };
    }

    const folder = DriveApp.getFolderById(folderId);
    const files = folder.getFilesByType(MimeType.PDF);

    let latestFile = null;
    let latestTime = null;
    let sameNameFile = null;

    while (files.hasNext()) {
      const file = files.next();
      const fileTime = file.getLastUpdated();

      // 同名ファイルを探す
      if (file.getName() === oldFileName) {
        sameNameFile = file;
      }

      // 最新のファイルを追跡
      if (!latestTime || fileTime > latestTime) {
        latestTime = fileTime;
        latestFile = file;
      }
    }

    // 同名ファイルがあればそれを優先、なければ最新のファイル
    const targetFile = sameNameFile || latestFile;

    if (!targetFile) {
      return { error: 'No PDF files found in folder' };
    }

    const isUpdated = targetFile.getId() !== oldFileId;

    return {
      success: true,
      fileId: targetFile.getId(),
      fileName: targetFile.getName(),
      folderId: folderId,
      modifiedTime: targetFile.getLastUpdated().toISOString(),
      isLatest: true,
      wasUpdated: isUpdated,
      oldFileId: isUpdated ? oldFileId : undefined
    };
  } catch (err) {
    return { error: 'Failed to get latest file: ' + err.message };
  }
}

// ExcelファイルをBase64で取得（フロントエンドでSheetJSパース用）
function fetchExcelAsBase64(fileId) {
  try {
    const file = DriveApp.getFileById(fileId);
    const fileName = file.getName();
    const mimeType = file.getMimeType();

    // Drive API v3で直接取得
    const url = 'https://www.googleapis.com/drive/v3/files/' + fileId + '?alt=media';
    const response = UrlFetchApp.fetch(url, {
      headers: { Authorization: 'Bearer ' + ScriptApp.getOAuthToken() },
      muteHttpExceptions: true
    });

    if (response.getResponseCode() !== 200) {
      return { error: 'Failed to fetch file: ' + response.getContentText() };
    }

    const blob = response.getBlob();
    const base64 = Utilities.base64Encode(blob.getBytes());

    return {
      success: true,
      fileId: fileId,
      fileName: fileName,
      mimeType: mimeType,
      base64: base64
    };
  } catch (err) {
    return { error: 'Failed to fetch Excel: ' + err.message };
  }
}

// スプレッドシートの全シート一覧を取得
// 施工体制台帳用に重要フィールドも抽出
function listSpreadsheetSheets(spreadsheetId) {
  try {
    const ss = SpreadsheetApp.openById(spreadsheetId);
    const sheets = ss.getSheets();

    const sheetList = sheets.map(sheet => {
      const range = sheet.getDataRange();
      const rowCount = range.getNumRows();
      const colCount = range.getNumColumns();

      // 先頭部分をプレビュー用に取得（10行×20列でフィールド抽出に十分なデータ）
      const previewRows = Math.min(10, rowCount);
      const previewCols = Math.min(20, colCount);
      const preview = sheet.getRange(1, 1, previewRows, previewCols).getDisplayValues();

      // 施工体制台帳用の重要フィールドを抽出
      const fields = extractImportantFields(preview);

      return {
        sheetId: sheet.getSheetId(),
        name: sheet.getName(),
        rowCount: rowCount,
        colCount: colCount,
        preview: preview.slice(0, 3).map(row => row.slice(0, 5)), // カード表示用は3行×5列
        fields: fields
      };
    });

    return {
      success: true,
      spreadsheetId: spreadsheetId,
      spreadsheetName: ss.getName(),
      sheets: sheetList
    };
  } catch (err) {
    return { error: 'Failed to list sheets: ' + err.message };
  }
}

// 施工体制台帳から重要フィールドを抽出
function extractImportantFields(data) {
  const fields = {
    officeName: null,      // 事業所名
    directorName: null,    // 所長名
    createdDate: null,     // 名簿作成日
    submittedDate: null,   // 提出日
    projectName: null      // 工事名（検証用）
  };

  if (!data || data.length === 0) return fields;

  // 各行・列をスキャンしてキーワードを探す
  for (let row = 0; row < data.length; row++) {
    for (let col = 0; col < data[row].length; col++) {
      const cell = String(data[row][col] || '').trim();
      const cellLower = cell.toLowerCase();

      // 事業所名・事業所の名称
      if ((cell.includes('事業所') || cell.includes('事業所の名称')) && !fields.officeName) {
        // 右隣または下のセルから値を取得
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < data[row + 1].length ? data[row + 1][col] : null;
        fields.officeName = rightValue || belowValue || null;
      }

      // 所長・現場代理人・監督員
      if ((cell.includes('所長') || cell.includes('現場代理人') || cell.includes('責任者')) && !fields.directorName) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < data[row + 1].length ? data[row + 1][col] : null;
        fields.directorName = rightValue || belowValue || null;
      }

      // 作成日（名簿の作成日、作成年月日）
      if ((cell.includes('作成') && (cell.includes('日') || cell.includes('年月'))) && !fields.createdDate) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < data[row + 1].length ? data[row + 1][col] : null;
        fields.createdDate = rightValue || belowValue || null;
      }

      // 提出日
      if (cell.includes('提出') && !fields.submittedDate) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < data[row + 1].length ? data[row + 1][col] : null;
        fields.submittedDate = rightValue || belowValue || null;
      }

      // 工事名称・工事名
      if ((cell === '工事名称' || cell === '工事名') && !fields.projectName) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < data[row + 1].length ? data[row + 1][col] : null;
        fields.projectName = rightValue || belowValue || null;
      }
    }
  }

  return fields;
}

// 外部スプレッドシートのデータを取得（AIチェック用）
// getDisplayValues()を使用して書式適用済みの値を取得
function fetchSpreadsheetData(spreadsheetId, gid) {
  try {
    const ss = SpreadsheetApp.openById(spreadsheetId);
    let sheet;

    if (gid) {
      // gidからシートを探す
      const sheets = ss.getSheets();
      sheet = sheets.find(s => s.getSheetId().toString() === gid);
      if (!sheet) {
        sheet = sheets[0];  // 見つからなければ最初のシート
      }
    } else {
      sheet = ss.getSheets()[0];
    }

    const sheetName = sheet.getName();
    const range = sheet.getDataRange();

    // getDisplayValues()で書式適用済みの値を取得
    const displayValues = range.getDisplayValues();

    // 空行を除外（先頭50行まで、または最初の連続した空行まで）
    let lastNonEmptyRow = 0;
    for (let i = 0; i < displayValues.length && i < 100; i++) {
      if (displayValues[i].some(cell => cell.trim() !== '')) {
        lastNonEmptyRow = i + 1;
      }
    }

    const trimmedData = displayValues.slice(0, lastNonEmptyRow);

    return {
      success: true,
      spreadsheetId: spreadsheetId,
      sheetName: sheetName,
      sheetId: sheet.getSheetId(),
      rowCount: trimmedData.length,
      colCount: trimmedData[0]?.length || 0,
      data: trimmedData,
      modifiedTime: ss.getLastUpdated?.() || new Date().toISOString()
    };
  } catch (err) {
    return { error: 'Failed to fetch spreadsheet: ' + err.message };
  }
}

// ドキュメントURLを更新（fileId変更時にProjectDataを更新）
function updateDocUrl(contractorId, docKey, newFileId) {
  try {
    const ss = SpreadsheetApp.getActiveSpreadsheet();
    let dataSheet = ss.getSheetByName(CONFIG.DATA_SHEET);
    if (!dataSheet) {
      return { error: 'ProjectData sheet not found' };
    }

    const data = dataSheet.getRange('A1').getValue();
    if (!data) {
      return { error: 'No project data found' };
    }

    const project = JSON.parse(data);

    // 対象のcontractorとdocを探してURLを更新
    let updated = false;
    for (const contractor of (project.contractors || [])) {
      if (contractor.id === contractorId && contractor.docs && contractor.docs[docKey]) {
        const newUrl = `https://drive.google.com/file/d/${newFileId}/view?usp=drivesdk`;
        contractor.docs[docKey].url = newUrl;
        updated = true;
        break;
      }
    }

    if (!updated) {
      return { error: `Document not found: ${contractorId}/${docKey}` };
    }

    // 更新したデータを保存
    dataSheet.getRange('A1').setValue(JSON.stringify(project, null, 2));

    return {
      success: true,
      message: `URL updated for ${contractorId}/${docKey}`,
      newFileId: newFileId
    };
  } catch (err) {
    return { error: 'Failed to update doc URL: ' + err.message };
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
