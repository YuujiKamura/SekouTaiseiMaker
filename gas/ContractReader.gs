/**
 * スプレッドシート読み取りWebアプリ
 * GAS経由でスプレッドシートの構造を取得
 */

/**
 * Webアプリのエントリーポイント
 * GET: ?id=スプレッドシートID でデータ取得
 */
function doGet(e) {
  try {
    const sheetId = e.parameter.id;
    if (!sheetId) {
      return ContentService.createTextOutput(JSON.stringify({
        error: true,
        message: 'スプレッドシートIDが必要です（?id=xxx）'
      })).setMimeType(ContentService.MimeType.JSON);
    }
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

