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
  HISTORY_SHEET: 'History'
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
    const data = loadProject();
    return jsonResponse(data);
  } catch (err) {
    return jsonResponse({ error: err.message });
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
