//! HTML report generator - Single file dashboard

use crate::analyzer::{CodebaseAnalysis, IssueCategory, Severity};
use crate::reporter::Reporter;
use anyhow::Result;

pub struct HtmlReporter;

impl Reporter for HtmlReporter {
    fn generate(analysis: &CodebaseAnalysis) -> Result<String> {
        let json_data = serde_json::to_string(analysis)?;

        Ok(format!(r##"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Codebase Health Dashboard</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0f172a; color: #e2e8f0; line-height: 1.6;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 20px; }}
        h1 {{ font-size: 1.8rem; margin-bottom: 10px; }}
        h2 {{ font-size: 1.3rem; margin: 20px 0 10px; color: #94a3b8; }}
        h3 {{ font-size: 1.1rem; margin: 15px 0 8px; }}

        .header {{
            background: linear-gradient(135deg, #1e293b 0%, #334155 100%);
            padding: 30px; border-radius: 12px; margin-bottom: 20px;
        }}
        .header-info {{ color: #94a3b8; font-size: 0.9rem; }}

        .score-card {{
            display: inline-flex; align-items: center; gap: 15px;
            background: #1e293b; padding: 20px 30px; border-radius: 12px;
            margin-top: 15px;
        }}
        .score {{ font-size: 3rem; font-weight: bold; }}
        .score.excellent {{ color: #22c55e; }}
        .score.good {{ color: #84cc16; }}
        .score.warning {{ color: #eab308; }}
        .score.poor {{ color: #f97316; }}
        .score.critical {{ color: #ef4444; }}
        .score-label {{ color: #94a3b8; }}

        .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 20px; }}
        .card {{
            background: #1e293b; border-radius: 12px; padding: 20px;
            border: 1px solid #334155;
        }}
        .card-title {{ font-size: 0.85rem; color: #64748b; text-transform: uppercase; margin-bottom: 10px; }}
        .card-value {{ font-size: 2rem; font-weight: bold; }}
        .card-sub {{ font-size: 0.85rem; color: #94a3b8; margin-top: 5px; }}

        .stats-row {{ display: flex; gap: 10px; flex-wrap: wrap; margin-top: 10px; }}
        .stat {{ background: #334155; padding: 8px 12px; border-radius: 6px; font-size: 0.85rem; }}

        .lang-bar {{ height: 8px; background: #334155; border-radius: 4px; overflow: hidden; display: flex; margin: 10px 0; }}
        .lang-segment {{ height: 100%; }}
        .lang-legend {{ display: flex; flex-wrap: wrap; gap: 10px; font-size: 0.8rem; }}
        .lang-item {{ display: flex; align-items: center; gap: 5px; }}
        .lang-dot {{ width: 10px; height: 10px; border-radius: 50%; }}

        .issue-list {{ max-height: 400px; overflow-y: auto; }}
        .issue {{
            background: #334155; padding: 12px; border-radius: 8px; margin-bottom: 8px;
            border-left: 3px solid;
        }}
        .issue.critical {{ border-color: #ef4444; }}
        .issue.high {{ border-color: #f97316; }}
        .issue.medium {{ border-color: #eab308; }}
        .issue.low {{ border-color: #22c55e; }}
        .issue-title {{ font-weight: 600; margin-bottom: 4px; display: flex; justify-content: space-between; align-items: center; }}
        .issue-file {{ font-size: 0.8rem; color: #94a3b8; font-family: monospace; }}
        .issue-desc {{ font-size: 0.85rem; color: #cbd5e1; margin-top: 5px; }}
        .issue-actions {{ display: flex; gap: 8px; margin-top: 8px; }}
        .copy-btn {{
            background: #3b82f6; color: white; border: none;
            padding: 6px 12px; border-radius: 6px; cursor: pointer;
            font-size: 0.8rem; font-weight: 500;
            transition: background 0.2s;
        }}
        .copy-btn:hover {{ background: #2563eb; }}
        .copy-btn:active {{ background: #1d4ed8; }}
        .copy-btn.copied {{ background: #22c55e; }}

        .badge {{
            display: inline-block; padding: 2px 8px; border-radius: 4px;
            font-size: 0.75rem; font-weight: 600;
        }}
        .badge.critical {{ background: #7f1d1d; color: #fca5a5; }}
        .badge.high {{ background: #7c2d12; color: #fdba74; }}
        .badge.medium {{ background: #713f12; color: #fde047; }}
        .badge.low {{ background: #14532d; color: #86efac; }}

        .tabs {{ display: flex; gap: 5px; margin-bottom: 15px; }}
        .tab {{
            padding: 8px 16px; border-radius: 6px; background: #334155;
            border: none; color: #94a3b8; cursor: pointer; font-size: 0.9rem;
        }}
        .tab.active {{ background: #3b82f6; color: white; }}

        .complexity-item {{
            display: flex; justify-content: space-between; align-items: center;
            padding: 8px 0; border-bottom: 1px solid #334155;
        }}
        .complexity-item:last-child {{ border-bottom: none; }}
        .complexity-name {{ font-family: monospace; font-size: 0.85rem; }}
        .complexity-value {{
            background: #334155; padding: 2px 8px; border-radius: 4px;
            font-family: monospace; font-size: 0.8rem;
        }}

        footer {{ text-align: center; padding: 30px; color: #64748b; font-size: 0.85rem; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Codebase Health Dashboard</h1>
            <div class="header-info">
                <div>Project: <strong id="project-path"></strong></div>
                <div>Analyzed: <span id="analyzed-at"></span></div>
            </div>
            <div class="score-card">
                <div class="score" id="health-score"></div>
                <div>
                    <div style="font-size: 1.2rem; font-weight: 600;">Health Score</div>
                    <div class="score-label" id="score-label"></div>
                </div>
            </div>
        </div>

        <div class="grid">
            <div class="card">
                <div class="card-title">Total Files</div>
                <div class="card-value" id="total-files"></div>
                <div class="card-sub" id="total-lines"></div>
            </div>
            <div class="card">
                <div class="card-title">Code Lines</div>
                <div class="card-value" id="code-lines"></div>
                <div class="card-sub" id="code-percent"></div>
            </div>
            <div class="card">
                <div class="card-title">Comments</div>
                <div class="card-value" id="comment-lines"></div>
                <div class="card-sub" id="comment-percent"></div>
            </div>
            <div class="card">
                <div class="card-title">Issues Found</div>
                <div class="card-value" id="total-issues"></div>
                <div class="stats-row" id="issue-badges"></div>
            </div>
        </div>

        <div style="margin-top: 20px;">
            <h2>Issues</h2>
            <div class="tabs">
                <button class="tab active" onclick="filterIssues('all')">All</button>
                <button class="tab" onclick="filterIssues('critical')">Critical</button>
                <button class="tab" onclick="filterIssues('high')">High</button>
                <button class="tab" onclick="filterIssues('medium')">Medium</button>
            </div>
            <div class="card">
                <div class="issue-list" id="issue-list"></div>
            </div>
        </div>

        <div style="margin-top: 20px;">
            <h2>Complexity</h2>
            <div class="card">
                <div class="stats-row" style="margin-bottom: 15px;">
                    <div class="stat">Avg: <strong id="avg-complexity"></strong></div>
                    <div class="stat">Max: <strong id="max-complexity"></strong></div>
                    <div class="stat">Functions: <strong id="total-functions"></strong></div>
                </div>
                <h3>Long Functions (>50 lines)</h3>
                <div id="long-functions"></div>
                <h3 style="margin-top: 15px;">Deeply Nested (>4 levels)</h3>
                <div id="deeply-nested"></div>
            </div>
        </div>

        <h2 style="margin-top: 20px;">Language Distribution</h2>
        <div class="card">
            <div class="lang-bar" id="lang-bar"></div>
            <div class="lang-legend" id="lang-legend"></div>
        </div>
    </div>

    <footer>
        Generated by codebase-health
    </footer>

    <script>
    const data = {json_data};

    const langColors = {{
        'rs': '#dea584', 'ts': '#3178c6', 'tsx': '#3178c6', 'js': '#f7df1e',
        'jsx': '#f7df1e', 'py': '#3776ab', 'go': '#00add8', 'java': '#b07219'
    }};

    function getScoreClass(score) {{
        if (score >= 90) return 'excellent';
        if (score >= 70) return 'good';
        if (score >= 50) return 'warning';
        if (score >= 30) return 'poor';
        return 'critical';
    }}

    function getScoreLabel(score) {{
        if (score >= 90) return 'Excellent';
        if (score >= 70) return 'Good';
        if (score >= 50) return 'Needs Improvement';
        if (score >= 30) return 'Poor';
        return 'Critical';
    }}

    function init() {{
        // Header
        document.getElementById('project-path').textContent = data.root_path;
        document.getElementById('analyzed-at').textContent = new Date(data.analyzed_at).toLocaleString();

        const scoreEl = document.getElementById('health-score');
        scoreEl.textContent = data.health_score;
        scoreEl.className = 'score ' + getScoreClass(data.health_score);
        document.getElementById('score-label').textContent = getScoreLabel(data.health_score);

        // Stats
        const stats = data.total_stats;
        document.getElementById('total-files').textContent = stats.total_files.toLocaleString();
        document.getElementById('total-lines').textContent = stats.total_lines.toLocaleString() + ' lines';
        document.getElementById('code-lines').textContent = stats.code_lines.toLocaleString();
        document.getElementById('code-percent').textContent =
            (stats.code_lines / stats.total_lines * 100).toFixed(1) + '% of total';
        document.getElementById('comment-lines').textContent = stats.comment_lines.toLocaleString();
        document.getElementById('comment-percent').textContent =
            (stats.comment_lines / stats.total_lines * 100).toFixed(1) + '% coverage';

        // Issues summary
        const issues = data.issues;
        document.getElementById('total-issues').textContent = issues.length;
        const counts = {{critical: 0, high: 0, medium: 0, low: 0}};
        issues.forEach(i => {{
            const sev = i.severity.toLowerCase();
            if (counts[sev] !== undefined) counts[sev]++;
        }});
        document.getElementById('issue-badges').innerHTML =
            `<span class="badge critical">${{counts.critical}} Critical</span>
             <span class="badge high">${{counts.high}} High</span>
             <span class="badge medium">${{counts.medium}} Medium</span>
             <span class="badge low">${{counts.low}} Low</span>`;

        // Languages
        const langs = Object.entries(data.file_stats).sort((a,b) => b[1].code_lines - a[1].code_lines);
        const totalCode = langs.reduce((sum, [,s]) => sum + s.code_lines, 0);
        const langBar = document.getElementById('lang-bar');
        const langLegend = document.getElementById('lang-legend');
        langs.forEach(([lang, s]) => {{
            const pct = (s.code_lines / totalCode * 100);
            const color = langColors[lang] || '#64748b';
            langBar.innerHTML += `<div class="lang-segment" style="width:${{pct}}%;background:${{color}}"></div>`;
            langLegend.innerHTML += `<div class="lang-item"><div class="lang-dot" style="background:${{color}}"></div>${{lang}} (${{s.file_count}} files, ${{pct.toFixed(1)}}%)</div>`;
        }});

        // Issues list
        renderIssues('all');

        // Complexity
        const cx = data.complexity;
        document.getElementById('avg-complexity').textContent = cx.avg_complexity.toFixed(2);
        document.getElementById('max-complexity').textContent = cx.max_complexity;
        document.getElementById('total-functions').textContent = cx.total_functions;

        renderComplexityList('long-functions', cx.long_functions);
        renderComplexityList('deeply-nested', cx.deeply_nested);
    }}

    let filteredIssues = [];

    function renderIssues(filter) {{
        const list = document.getElementById('issue-list');
        let issues = data.issues;
        if (filter !== 'all') {{
            issues = issues.filter(i => i.severity.toLowerCase() === filter);
        }}
        filteredIssues = issues;
        list.innerHTML = issues.slice(0, 50).map((i, idx) => {{
            const originalIdx = data.issues.findIndex(orig => 
                orig.file === i.file && 
                orig.line === i.line && 
                orig.title === i.title
            );
            return `
            <div class="issue ${{i.severity.toLowerCase()}}">
                <div class="issue-title">
                    <span>${{i.title}}</span>
                    <button class="copy-btn" onclick="copyTaskToClipboard(${{originalIdx >= 0 ? originalIdx : idx}})" data-issue-idx="${{originalIdx >= 0 ? originalIdx : idx}}">
                        📋 コピー
                    </button>
                </div>
                <div class="issue-file">${{i.file}}${{i.line ? ':' + i.line : ''}}</div>
                ${{i.description ? `<div class="issue-desc">${{i.description}}</div>` : ''}}
            </div>
        `;
        }}).join('');
        if (issues.length > 50) {{
            list.innerHTML += `<div style="padding:10px;color:#64748b">...and ${{issues.length - 50}} more</div>`;
        }}
    }}

    function filterIssues(filter) {{
        document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
        event.target.classList.add('active');
        renderIssues(filter);
    }}

    function renderComplexityList(id, items) {{
        const el = document.getElementById(id);
        if (!items || items.length === 0) {{
            el.innerHTML = '<div style="color:#64748b;font-size:0.85rem">None</div>';
            return;
        }}
        el.innerHTML = items.slice(0, 10).map(item => `
            <div class="complexity-item">
                <span class="complexity-name">${{item.split('/').pop()}}</span>
            </div>
        `).join('');
    }}

    function generateClaudeTask(issue) {{
        const severityNames = {{
            'critical': 'Critical',
            'high': 'High',
            'medium': 'Medium',
            'low': 'Low',
            'info': 'Info'
        }};
        const categoryNames = {{
            'Security': 'Security',
            'CodeQuality': 'Code Quality',
            'Performance': 'Performance',
            'Maintainability': 'Maintainability',
            'Documentation': 'Documentation',
            'Testing': 'Testing',
            'BestPractice': 'Best Practice'
        }};

        const priority = issue.severity === 'critical' ? 'P1' :
                        issue.severity === 'high' ? 'P2' :
                        issue.severity === 'medium' ? 'P3' :
                        issue.severity === 'low' ? 'P4' : 'P5';

        let task = `# コード改善タスク

**優先度:** ${{priority}} (${{severityNames[issue.severity]}})
**カテゴリ:** ${{categoryNames[issue.category] || issue.category}}
**ファイル:** \`${{issue.file}}\`
${{issue.line ? `**行番号:** ${{issue.line}}\n` : ''}}

## 問題の説明

${{issue.title}}

${{issue.description ? issue.description : ''}}

## 改善提案

${{issue.suggestion || '該当箇所を確認し、適切な修正を実施してください。'}}

## 受け入れ基準

- [ ] 問題が解決されている
- [ ] コードが正常にコンパイル/実行できる
- [ ] 既存のテストが通過する
- [ ] コードの可読性が向上している

## 実装時のヒント

${{issue.category === 'Security' ? '- セキュリティベストプラクティスに従う\n- 機密情報が適切に扱われているか確認する' : ''}}
${{issue.category === 'CodeQuality' ? '- コードの可読性を向上させる\n- エラーハンドリングを追加する' : ''}}
${{issue.category === 'Performance' ? '- パフォーマンスプロファイリングを検討する\n- キャッシュやメモ化を検討する' : ''}}
${{issue.category === 'Maintainability' ? '- 変更は最小限に留める\n- 複雑なロジックにはコメントを追加する' : ''}}
${{issue.category === 'Documentation' ? '- 公開APIにはドキュメントコメントを追加する\n- 例を含める場合は追加する' : ''}}
${{issue.category === 'Testing' ? '- 新しいコードにはユニットテストを追加する\n- エッジケースを考慮する' : ''}}
${{issue.category === 'BestPractice' ? '- プロジェクトのコーディング規約に従う\n- CONTRIBUTINGガイドを確認する' : ''}}

---

## Claudeへの指示

このタスクを完了する際は：

1. まず、影響を受けるファイルを読み、現在の実装を理解する
2. 特定の問題に対応する最小限の変更を行う
3. すべての受け入れ基準が満たされていることを確認する
4. 関連するテストを実行して変更を検証する
5. このタスクIDを参照する明確なメッセージでコミットする

\`\`\`
git commit -m "fix: ${{issue.title}} in ${{issue.file}}${{issue.line ? ':' + issue.line : ''}}"
\`\`\`
`;

        return task;
    }}

    async function copyTaskToClipboard(issueIdx) {{
        const issue = data.issues[issueIdx];
        if (!issue) return;

        const taskText = generateClaudeTask(issue);

        try {{
            await navigator.clipboard.writeText(taskText);
            
            // ボタンの状態を更新
            const btn = document.querySelector(`[data-issue-idx="${{issueIdx}}"]`);
            if (btn) {{
                const originalText = btn.textContent;
                btn.textContent = '✓ コピー済み';
                btn.classList.add('copied');
                setTimeout(() => {{
                    btn.textContent = originalText;
                    btn.classList.remove('copied');
                }}, 2000);
            }}
        }} catch (err) {{
            console.error('クリップボードへのコピーに失敗しました:', err);
            alert('クリップボードへのコピーに失敗しました。手動でコピーしてください。');
        }}
    }}

    init();
    </script>
</body>
</html>"##))
    }
}
