/**
 * 契約書スプレッドシート読み取りスクリプト
 * GAS経由でスプレッドシートの構造を取得
 */

// 契約書スプレッドシートID
const CONTRACT_SHEET_ID = 'REDACTED_SHEET_ID';

/**
 * 契約書スプレッドシートの構造を取得
 */
function getContractSheetStructure() {
  const ss = SpreadsheetApp.openById(CONTRACT_SHEET_ID);
  const sheets = ss.getSheets();

  const result = {
    spreadsheetName: ss.getName(),
    sheets: []
  };

  for (const sheet of sheets) {
    const sheetName = sheet.getName();
    const data = sheet.getDataRange().getValues();
    const lastRow = sheet.getLastRow();
    const lastCol = sheet.getLastColumn();

    result.sheets.push({
      name: sheetName,
      rows: lastRow,
      cols: lastCol,
      data: data.slice(0, 50) // 最初の50行
    });
  }

  console.log(JSON.stringify(result, null, 2));
  return result;
}

/**
 * シート一覧を取得
 */
function listSheets() {
  const ss = SpreadsheetApp.openById(CONTRACT_SHEET_ID);
  const sheets = ss.getSheets();

  console.log('📊 スプレッドシート: ' + ss.getName());
  console.log('📑 シート一覧:');

  for (const sheet of sheets) {
    console.log('   - ' + sheet.getName() + ' (' + sheet.getLastRow() + '行)');
  }

  return sheets.map(s => s.getName());
}

/**
 * 特定シートの内容を取得
 */
function getSheetContent(sheetName) {
  const ss = SpreadsheetApp.openById(CONTRACT_SHEET_ID);
  const sheet = ss.getSheetByName(sheetName);

  if (!sheet) {
    throw new Error('シートが見つかりません: ' + sheetName);
  }

  const data = sheet.getDataRange().getValues();

  console.log('## シート: ' + sheetName);
  console.log('行数: ' + data.length);

  // 最初の30行を表示
  for (let i = 0; i < Math.min(30, data.length); i++) {
    console.log((i + 1) + ': ' + data[i].join(' | '));
  }

  return data;
}

/**
 * 契約書データをJSON形式で出力
 */
function exportContractData() {
  const ss = SpreadsheetApp.openById(CONTRACT_SHEET_ID);
  const sheets = ss.getSheets();

  const exportData = {
    exportedAt: new Date().toISOString(),
    spreadsheetId: CONTRACT_SHEET_ID,
    spreadsheetName: ss.getName(),
    sheets: {}
  };

  for (const sheet of sheets) {
    const sheetName = sheet.getName();
    const data = sheet.getDataRange().getValues();

    exportData.sheets[sheetName] = {
      rows: data.length,
      cols: data[0] ? data[0].length : 0,
      data: data
    };
  }

  console.log(JSON.stringify(exportData, null, 2));
  return exportData;
}
