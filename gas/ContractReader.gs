/**
 * スプレッドシート読み取りWebアプリ
 * GAS経由でスプレッドシートの構造を取得
 */

/**
 * Webアプリのエントリーポイント
 * GET: ?id=スプレッドシートID でデータ取得
 * GET: ?folder=フォルダID でフォルダ内一覧取得
 */
function doGet(e) {
  try {
    const folderId = e.parameter.folder;
    const sheetId = e.parameter.id;

    // フォルダ内のスプレッドシート一覧
    if (folderId) {
      const list = listSpreadsheetsInFolder(folderId);
      return ContentService.createTextOutput(JSON.stringify(list))
        .setMimeType(ContentService.MimeType.JSON);
    }

    // 個別スプレッドシート取得
    if (sheetId) {
      const data = readSpreadsheet(sheetId);
      return ContentService.createTextOutput(JSON.stringify(data))
        .setMimeType(ContentService.MimeType.JSON);
    }

    return ContentService.createTextOutput(JSON.stringify({
      error: true,
      message: 'パラメータが必要です（?folder=xxx または ?id=xxx）'
    })).setMimeType(ContentService.MimeType.JSON);

  } catch (error) {
    return ContentService.createTextOutput(JSON.stringify({
      error: true,
      message: error.message
    })).setMimeType(ContentService.MimeType.JSON);
  }
}

/**
 * フォルダ内のスプレッドシート一覧を取得
 */
function listSpreadsheetsInFolder(folderId) {
  const folder = DriveApp.getFolderById(folderId);
  const files = folder.getFilesByType(MimeType.GOOGLE_SHEETS);

  const spreadsheets = [];
  while (files.hasNext()) {
    const file = files.next();
    spreadsheets.push({
      id: file.getId(),
      name: file.getName(),
      lastUpdated: file.getLastUpdated().toISOString()
    });
  }

  return {
    folder_id: folderId,
    folder_name: folder.getName(),
    spreadsheets: spreadsheets
  };
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

