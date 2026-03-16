#!/usr/bin/env node
"use strict";

const { execSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");
const https = require("https");

const VERSION = require("./package.json").version;
const REPO = "Rothschildiuk/context-pack";
const BIN_DIR = path.join(__dirname, "bin");
const BIN_PATH = path.join(BIN_DIR, "context-pack");

const PLATFORM_MAP = {
  "darwin-arm64": `context-pack-v${VERSION}-aarch64-apple-darwin.tar.gz`,
  "darwin-x64": `context-pack-v${VERSION}-x86_64-apple-darwin.tar.gz`,
  "linux-x64": `context-pack-v${VERSION}-x86_64-unknown-linux-gnu.tar.gz`,
};

function getPlatformKey() {
  const platform = os.platform();
  const arch = os.arch();
  return `${platform}-${arch}`;
}

function fetch(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return fetch(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function install() {
  const key = getPlatformKey();
  const asset = PLATFORM_MAP[key];
  if (!asset) {
    console.error(`context-pack: unsupported platform ${key}`);
    console.error(`Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`);
    process.exit(1);
  }

  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;
  console.log(`Downloading context-pack v${VERSION} for ${key}...`);

  const tarball = await fetch(url);

  const tmpFile = path.join(os.tmpdir(), asset);
  fs.writeFileSync(tmpFile, tarball);

  fs.mkdirSync(BIN_DIR, { recursive: true });
  execSync(`tar -xzf "${tmpFile}" -C "${BIN_DIR}" context-pack`, { stdio: "inherit" });
  fs.chmodSync(BIN_PATH, 0o755);
  fs.unlinkSync(tmpFile);

  console.log(`context-pack v${VERSION} installed successfully.`);
}

install().catch((err) => {
  console.error(`Failed to install context-pack: ${err.message}`);
  process.exit(1);
});
