#!/usr/bin/env node

const { execSync } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");

const PACKAGE = require("../package.json");
const VERSION = PACKAGE.version;
const REPO = "mstuart/code-memory";
const BINARY_NAME = "code-memory";

function getPlatformTarget() {
  const platform = os.platform();
  const arch = os.arch();

  const targets = {
    "darwin-arm64": "aarch64-apple-darwin",
    "darwin-x64": "x86_64-apple-darwin",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "linux-x64": "x86_64-unknown-linux-gnu",
    "win32-x64": "x86_64-pc-windows-msvc",
  };

  const key = `${platform}-${arch}`;
  const target = targets[key];

  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`Supported platforms: ${Object.keys(targets).join(", ")}`);
    process.exit(1);
  }

  return { arch, platform, target };
}

function getDownloadUrl(target) {
  const archive = target.includes("windows") ? "zip" : "tar.gz";
  return `https://github.com/${REPO}/releases/download/v${VERSION}/${BINARY_NAME}-v${VERSION}-${target}.${archive}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const handler = (response) => {
      if (
        response.statusCode >= 300 &&
        response.statusCode < 400 &&
        response.headers.location
      ) {
        const redirectUrl = response.headers.location;
        const client = redirectUrl.startsWith("https") ? https : http;
        client.get(redirectUrl, handler).on("error", reject);
        return;
      }

      if (response.statusCode !== 200) {
        reject(
          new Error(
            `Download failed with status ${response.statusCode}: ${url}`
          )
        );
        return;
      }

      const chunks = [];
      response.on("data", (chunk) => chunks.push(chunk));
      response.on("end", () => resolve(Buffer.concat(chunks)));
      response.on("error", reject);
    };

    https.get(url, handler).on("error", reject);
  });
}

function extractTarGz(buffer, destDir) {
  const tmpFile = path.join(os.tmpdir(), `code-memory-${Date.now()}.tar.gz`);
  fs.writeFileSync(tmpFile, buffer);

  try {
    execSync(`tar xzf "${tmpFile}" -C "${destDir}"`, { stdio: "pipe" });
  } finally {
    try {
      fs.unlinkSync(tmpFile);
    } catch {
      // The temporary archive may already have been removed.
    }
  }
}

function extractZip(buffer, destDir) {
  const tmpFile = path.join(os.tmpdir(), `code-memory-${Date.now()}.zip`);
  fs.writeFileSync(tmpFile, buffer);

  try {
    execSync(`unzip -o "${tmpFile}" -d "${destDir}"`, { stdio: "pipe" });
  } finally {
    try {
      fs.unlinkSync(tmpFile);
    } catch {
      // The temporary archive may already have been removed.
    }
  }
}

async function install() {
  const { target, platform } = getPlatformTarget();
  const binDir = path.join(
    path.dirname(require.resolve("../package.json")),
    "bin"
  );
  const extractedName =
    platform === "win32" ? `${BINARY_NAME}.exe` : BINARY_NAME;
  const extractedPath = path.join(binDir, extractedName);
  const binPath = path.join(
    binDir,
    platform === "win32" ? `${BINARY_NAME}-bin.exe` : `${BINARY_NAME}-bin`
  );

  // Check if binary already exists
  if (fs.existsSync(binPath)) {
    console.log(`code-memory binary already installed at ${binPath}`);
    return;
  }

  const url = getDownloadUrl(target);
  console.log(`Downloading code-memory v${VERSION} for ${target}...`);
  console.log(`  URL: ${url}`);

  try {
    const data = await download(url);
    console.log(`  Downloaded ${(data.length / 1024 / 1024).toFixed(1)} MB`);

    // Extract
    fs.mkdirSync(binDir, { recursive: true });

    if (target.includes("windows")) {
      extractZip(data, binDir);
    } else {
      extractTarGz(data, binDir);
    }

    if (!fs.existsSync(extractedPath)) {
      throw new Error(`archive did not contain ${extractedName}`);
    }

    if (extractedPath !== binPath) {
      fs.renameSync(extractedPath, binPath);
    }

    if (platform !== "win32") {
      fs.chmodSync(binPath, 0o755);
    }

    console.log(`  Installed to ${binPath}`);
  } catch (error) {
    console.warn(`\nFailed to download pre-built binary: ${error.message}`);
    console.warn("\nTry reinstalling: npm install -g code-memory");
    console.warn("\nOr build from source instead:");
    console.warn(
      "  cargo install --git https://github.com/mstuart/code-memory code-memory"
    );
    console.warn("\nOr download manually from:");
    console.warn(`  https://github.com/${REPO}/releases`);
  }
}

install().catch((error) => {
  console.error("Installation failed:", error.message);
  process.exit(1);
});
