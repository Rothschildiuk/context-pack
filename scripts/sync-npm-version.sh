#!/usr/bin/env bash
# Syncs npm/package.json and npm/server.json versions from Cargo.toml
set -euo pipefail

version=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

for file in npm/package.json npm/server.json; do
  if [ -f "$file" ]; then
    tmp=$(mktemp)
    node -e "
      const fs = require('fs');
      const pkg = JSON.parse(fs.readFileSync('$file', 'utf8'));
      pkg.version = '$version';
      if (pkg.packages) pkg.packages.forEach(p => p.version = '$version');
      fs.writeFileSync('$file', JSON.stringify(pkg, null, 2) + '\n');
    "
    echo "Updated $file to v$version"
  fi
done
