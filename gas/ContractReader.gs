/**
 * スプレッドシート読み取りWebアプリ
 * GAS経由でスプレッドシートの構造を取得
 */

// デフォルトの契約書スプレッドシートID
const DEFAULT_SHEET_ID = 'REDACTED_SHEET_ID';

/**
 * Webアプリのエントリーポイント
 * GET: ?id=スプレッドシートID でデータ取得
 */
function doGet(e) {
  try {
    const sheetId = e.parameter.id || DEFAULT_SHEET_ID;
    const data = readSpreadsheet(sheetId);

    return ContentService.createTextOutput(JSON.stringify(data))
      .setMimeType(ContentService.MimeType.JSON);
  } catch (error) {
    return ContentService.createTextOutput(JSON.stringify({
      error: true,
      message: error.message
    })).setMimeType(ContentService.MimeType.JSON);
  }
}

/**
 * スプレッドシートを読み取り
 */
function readSpreadsheet(sheetId) {
  const ss = SpreadsheetApp.openById(sheetId);
  const sheets = ss.getSheets();

  const result = {
    spreadsheet_name: ss.getName(),
    spreadsheet_id: sheetId,
    sheets: {}
  };

  for (const sheet of sheets) {
    const sheetName = sheet.getName();
    const data = sheet.getDataRange().getValues();

    // 空セルを空文字列に変換
    const cleanData = data.map(row =>
      row.map(cell => cell === null || cell === undefined ? '' : String(cell))
    );

    result.sheets[sheetName] = {
      rows: data.length,
      data: cleanData
    };
  }

  return result;
}

/**
 * テスト用: デフォルトシートの構造を取得
 */
function testReadDefault() {
  const data = readSpreadsheet(DEFAULT_SHEET_ID);
  console.log(JSON.stringify(data, null, 2));
  return data;
}
