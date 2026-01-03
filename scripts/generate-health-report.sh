#!/bin/bash
if [ -f "tools/codebase-health/target/release/codebase-health" ]; then
    ./tools/codebase-health/target/release/codebase-health analyze --format html --output dist/health-report.html
else
    echo "codebase-health tool not found"
fi

