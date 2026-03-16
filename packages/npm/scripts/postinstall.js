#!/usr/bin/env node

const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");

const packageJson = require("../package.json");

function platformTarget() {
  if (process.platform === "linux" && process.arch === "x64") {
    return { target: "x86_64-unknown-linux-gnu", ext: "" };
  }
  if (process.platform === "linux" && process.arch === "arm64") {
    return { target: "aarch64-unknown-linux-gnu", ext: "" };
  }
  if (process.platform === "darwin" && process.arch === "x64") {
    return { target: "x86_64-apple-darwin", ext: "" };
  }
  if (process.platform === "darwin" && process.arch === "arm64") {
    return { target: "aarch64-apple-darwin", ext: "" };
  }
  if (process.platform === "win32" && process.arch === "x64") {
    return { target: "x86_64-pc-windows-msvc", ext: ".exe" };
  }
  return null;
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (response) => {
        if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400) {
          if (!response.headers.location) {
            reject(new Error(`redirect without location for ${url}`));
            return;
          }
          download(response.headers.location, destination).then(resolve, reject);
          return;
        }
        if (response.statusCode !== 200) {
          reject(new Error(`download failed: ${response.statusCode} ${response.statusMessage}`));
          return;
        }
        const file = fs.createWriteStream(destination, { mode: 0o755 });
        response.pipe(file);
        file.on("finish", () => file.close(resolve));
        file.on("error", reject);
      })
      .on("error", reject);
  });
}

async function main() {
  const target = platformTarget();
  if (!target) {
    console.warn(`Skipping binary download for unsupported platform ${os.platform()} ${os.arch()}.`);
    return;
  }

  const vendorDir = path.join(__dirname, "..", "vendor");
  fs.mkdirSync(vendorDir, { recursive: true });

  const binaryName = `clawlegion-v${packageJson.version}-${target.target}${target.ext}`;
  const url = `https://github.com/clawlegion/clawlegion/releases/download/v${packageJson.version}/${binaryName}`;
  const destination = path.join(vendorDir, `clawlegion${target.ext}`);
  await download(url, destination);
}

main().catch((error) => {
  console.error(`Failed to install clawlegion binary: ${error.message}`);
  process.exit(1);
});
