/**
 * ファイルタイプ判定のテスト
 */
import { describe, it, expect } from 'vitest';

// Rust側のdetect_file_type相当のロジックをTypeScriptで再現
function detectFileType(url: string): 'excel' | 'spreadsheet' | 'pdf' | 'image' | 'doc' | 'unknown' {
  const urlLower = url.toLowerCase();

  // Google Spreadsheetとして開かれているExcelファイル（rtpof=true）
  if (urlLower.includes('docs.google.com/spreadsheets') && urlLower.includes('rtpof=true')) {
    return 'excel';
  } else if (urlLower.includes('docs.google.com/spreadsheets')) {
    return 'spreadsheet';
  } else if (urlLower.includes('docs.google.com/document')) {
    return 'doc';
  } else if (urlLower.includes('drive.google.com/file')) {
    return 'pdf';
  } else if (urlLower.endsWith('.pdf')) {
    return 'pdf';
  } else if (urlLower.endsWith('.xlsx') || urlLower.endsWith('.xls')) {
    return 'excel';
  } else if (urlLower.endsWith('.png') || urlLower.endsWith('.jpg') || urlLower.endsWith('.jpeg')) {
    return 'image';
  } else {
    return 'unknown';
  }
}

// spreadsheet_viewer.rsのis_excel_compat相当
function isExcelCompat(url: string): boolean {
  return url.includes('rtpof=true');
}

// spreadsheet_viewer.rsのextract_spreadsheet_id相当
function extractSpreadsheetId(url: string): string | null {
  const match = url.match(/\/d\/([a-zA-Z0-9-_]+)/);
  return match ? match[1] : null;
}

describe('ファイルタイプ判定', () => {
  describe('detectFileType', () => {
    it('Google Spreadsheet URL を判定', () => {
      const url = 'https://docs.google.com/spreadsheets/d/1abc123/edit#gid=0';
      expect(detectFileType(url)).toBe('spreadsheet');
    });

    it('rtpof=true を含む Excel ファイル URL を判定', () => {
      const url = 'https://docs.google.com/spreadsheets/d/1kmifk2AucPATyt0S6_phNr5OB8uEFhiQ/edit?rtpof=true&sd=true#gid=1234';
      expect(detectFileType(url)).toBe('excel');
    });

    it('.xlsx ファイルを判定', () => {
      const url = 'https://example.com/file.xlsx';
      expect(detectFileType(url)).toBe('excel');
    });

    it('.xls ファイルを判定', () => {
      const url = 'https://example.com/file.xls';
      expect(detectFileType(url)).toBe('excel');
    });

    it('Google Drive PDF を判定', () => {
      const url = 'https://drive.google.com/file/d/1abc123/view';
      expect(detectFileType(url)).toBe('pdf');
    });

    it('.pdf ファイルを判定', () => {
      const url = 'https://example.com/document.pdf';
      expect(detectFileType(url)).toBe('pdf');
    });
  });

  describe('isExcelCompat', () => {
    it('rtpof=true を含む URL は true', () => {
      const url = 'https://docs.google.com/spreadsheets/d/1abc/edit?rtpof=true&sd=true';
      expect(isExcelCompat(url)).toBe(true);
    });

    it('rtpof=true を含まない URL は false', () => {
      const url = 'https://docs.google.com/spreadsheets/d/1abc/edit#gid=0';
      expect(isExcelCompat(url)).toBe(false);
    });
  });

  describe('extractSpreadsheetId', () => {
    it('スプレッドシート URL から ID を抽出', () => {
      const url = 'https://docs.google.com/spreadsheets/d/1kmifk2AucPATyt0S6_phNr5OB8uEFhiQ/edit#gid=0';
      expect(extractSpreadsheetId(url)).toBe('1kmifk2AucPATyt0S6_phNr5OB8uEFhiQ');
    });

    it('Excel ファイル URL から ID を抽出', () => {
      const url = 'https://docs.google.com/spreadsheets/d/1abc123/edit?rtpof=true';
      expect(extractSpreadsheetId(url)).toBe('1abc123');
    });
  });

  describe('AIチェックURL構築', () => {
    it('スプレッドシートの場合は isExcel パラメータなし', () => {
      const url = 'https://docs.google.com/spreadsheets/d/1abc123/edit#gid=0';
      const id = extractSpreadsheetId(url);
      const isExcel = isExcelCompat(url);

      let checkUrl = `mode=spreadsheet-check&spreadsheetId=${id}`;
      if (isExcel) {
        checkUrl += `&isExcel=true&fileId=${id}`;
      }

      expect(checkUrl).toBe('mode=spreadsheet-check&spreadsheetId=1abc123');
      expect(checkUrl).not.toContain('isExcel=true');
    });

    it('Excel ファイルの場合は isExcel=true と fileId を追加', () => {
      const url = 'https://docs.google.com/spreadsheets/d/1abc123/edit?rtpof=true&sd=true';
      const id = extractSpreadsheetId(url);
      const isExcel = isExcelCompat(url);

      let checkUrl = `mode=spreadsheet-check&spreadsheetId=${id}`;
      if (isExcel) {
        checkUrl += `&isExcel=true&fileId=${id}`;
      }

      expect(checkUrl).toBe('mode=spreadsheet-check&spreadsheetId=1abc123&isExcel=true&fileId=1abc123');
      expect(checkUrl).toContain('isExcel=true');
      expect(checkUrl).toContain('fileId=1abc123');
    });
  });
});
