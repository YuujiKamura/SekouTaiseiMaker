#!/usr/bin/env node
/**
 * ヘルスダッシュボード更新用の開発サーバー
 * 
 * 使用方法:
 *   node scripts/health-report-server.js
 * 
 * ブラウザから http://localhost:8081/update にアクセスすると
 * ヘルスレポートが更新されます
 */

const http = require('http');
const { exec } = require('child_process');
const path = require('path');
const fs = require('fs');

const PORT = 8081;
const EXE_PATH_WIN = path.join(__dirname, '..', 'tools', 'codebase-health', 'target', 'release', 'codebase-health.exe');
const EXE_PATH_UNIX = path.join(__dirname, '..', 'tools', 'codebase-health', 'target', 'release', 'codebase-health');
const OUTPUT_PATH = path.join(__dirname, '..', 'dist', 'health-report.html');

const isWindows = process.platform === 'win32';
const exePath = isWindows ? EXE_PATH_WIN : EXE_PATH_UNIX;

function generateHealthReport() {
  return new Promise((resolve, reject) => {
    if (!fs.existsSync(exePath)) {
      reject(new Error(`codebase-health tool not found at ${exePath}`));
      return;
    }

    const command = `"${exePath}" analyze --format html --output "${OUTPUT_PATH}"`;
    exec(command, (error, stdout, stderr) => {
      if (error) {
        reject(error);
        return;
      }
      resolve({ stdout, stderr });
    });
  });
}

const server = http.createServer(async (req, res) => {
  // CORSヘッダーを追加
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  if (req.method === 'OPTIONS') {
    res.writeHead(200);
    res.end();
    return;
  }

  if (req.url === '/update' && (req.method === 'GET' || req.method === 'POST')) {
    try {
      console.log('Generating health report...');
      await generateHealthReport();
      console.log('Health report generated successfully');
      
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ 
        success: true, 
        message: 'Health report updated successfully',
        output: OUTPUT_PATH
      }));
    } catch (error) {
      console.error('Error generating health report:', error);
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ 
        success: false, 
        error: error.message 
      }));
    }
  } else if (req.url === '/status') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ 
      status: 'running',
      exePath: exePath,
      exeExists: fs.existsSync(exePath),
      outputPath: OUTPUT_PATH
    }));
  } else {
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('Not Found');
  }
});

server.listen(PORT, () => {
  console.log(`Health report update server running on http://localhost:${PORT}`);
  console.log(`Access http://localhost:${PORT}/update to generate health report`);
  console.log(`Access http://localhost:${PORT}/status to check server status`);
});

