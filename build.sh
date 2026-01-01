#!/bin/bash
# Full build script

set -e

echo "Building React app..."
cd react-app
npm run build
cd ..

echo "Building Trunk..."
trunk build

echo "Copying React app to dist/editor..."
mkdir -p dist/editor
cp -r react-app/dist/* dist/editor/

echo "Build complete!"
