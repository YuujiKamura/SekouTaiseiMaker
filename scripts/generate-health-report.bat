@echo off
if exist "tools\codebase-health\target\release\codebase-health.exe" (
    tools\codebase-health\target\release\codebase-health.exe analyze --format html --output dist\health-report.html
) else (
    echo codebase-health tool not found
)

