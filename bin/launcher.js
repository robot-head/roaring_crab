#!/usr/bin/env node
// roaring-crab cross-platform hook launcher.
//
// Invoked by hooks.json as:
//     node "${CLAUDE_PLUGIN_ROOT}/bin/launcher.js" <HookEvent>
//
// Detects the current platform + arch, locates the matching prebuilt
// `roaring-crab` binary under `${CLAUDE_PLUGIN_ROOT}/bin/<platform>/`,
// and execs it with `--event <HookEvent>`.
//
// If the binary isn't present locally yet, downloads the platform
// archive from this repo's latest GitHub Release, extracts it, and
// then runs. The download happens once per plugin install per platform.
//
// All failure paths exit 0 silently so a missing/unsupported platform
// or a network hiccup never surfaces as a hook error in Claude Code.

const path = require('path');
const fs = require('fs');
const os = require('os');
const https = require('https');
const { spawn, execSync } = require('child_process');

const REPO = 'robot-head/roaring_crab';
const RELEASE = 'latest'; // resolves via /releases/latest/download/<asset>

const PLATFORMS = {
  'win32-x64': {
    dir: 'windows-x86_64',
    exe: 'roaring-crab.exe',
    daemon: 'roaring-crabd.exe',
    archive: 'roaring-crab-windows-x86_64.zip',
  },
  'linux-x64': {
    dir: 'linux-x86_64',
    exe: 'roaring-crab',
    daemon: 'roaring-crabd',
    archive: 'roaring-crab-linux-x86_64.tar.gz',
  },
  // linux-arm64 is intentionally omitted; no prebuilt archive is published
  // for that platform yet. (cross-compile via `cross` needs ALSA headers
  // for the target arch in its Docker image, which isn't set up yet.)
  'darwin-x64': {
    dir: 'macos-x86_64',
    exe: 'roaring-crab',
    daemon: 'roaring-crabd',
    archive: 'roaring-crab-macos-x86_64.tar.gz',
  },
  'darwin-arm64': {
    dir: 'macos-aarch64',
    exe: 'roaring-crab',
    daemon: 'roaring-crabd',
    archive: 'roaring-crab-macos-aarch64.tar.gz',
  },
};

function fetchToFile(url, dest, redirectsLeft = 5) {
  return new Promise((resolve, reject) => {
    const req = https.get(url, { headers: { 'User-Agent': 'roaring-crab-launcher' } }, (res) => {
      if (
        res.statusCode >= 300 &&
        res.statusCode < 400 &&
        res.headers.location &&
        redirectsLeft > 0
      ) {
        res.resume();
        return fetchToFile(res.headers.location, dest, redirectsLeft - 1).then(
          resolve,
          reject
        );
      }
      if (res.statusCode !== 200) {
        res.resume();
        return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
      }
      const out = fs.createWriteStream(dest);
      res.pipe(out);
      out.on('finish', () => out.close(() => resolve()));
      out.on('error', reject);
    });
    req.on('error', reject);
    req.setTimeout(30000, () => req.destroy(new Error('download timeout')));
  });
}

function extractArchive(archivePath, destDir) {
  if (archivePath.endsWith('.zip')) {
    if (process.platform === 'win32') {
      execSync(
        `powershell -NoProfile -Command "Expand-Archive -Force -LiteralPath '${archivePath}' -DestinationPath '${destDir}'"`,
        { stdio: 'ignore' }
      );
    } else {
      execSync(`unzip -o "${archivePath}" -d "${destDir}"`, { stdio: 'ignore' });
    }
  } else if (archivePath.endsWith('.tar.gz') || archivePath.endsWith('.tgz')) {
    execSync(`tar -xzf "${archivePath}" -C "${destDir}"`, { stdio: 'ignore' });
  } else {
    throw new Error(`unknown archive type: ${archivePath}`);
  }
}

async function ensureBinary(info, binDir, binPath) {
  if (fs.existsSync(binPath)) return true;
  try {
    fs.mkdirSync(binDir, { recursive: true });
    const url = `https://github.com/${REPO}/releases/${RELEASE}/download/${info.archive}`;
    const tmpPath = path.join(os.tmpdir(), `rc-${Date.now()}-${info.archive}`);
    await fetchToFile(url, tmpPath);
    extractArchive(tmpPath, binDir);
    try {
      fs.unlinkSync(tmpPath);
    } catch (_) {}
    if (!fs.existsSync(binPath)) return false;
    if (process.platform !== 'win32') {
      try {
        fs.chmodSync(binPath, 0o755);
      } catch (_) {}
      try {
        fs.chmodSync(path.join(binDir, info.daemon), 0o755);
      } catch (_) {}
    }
    return true;
  } catch (_) {
    return false;
  }
}

async function main() {
  const event = process.argv[2];
  if (!event) return;

  const root =
    process.env.CLAUDE_PLUGIN_ROOT ||
    path.resolve(path.dirname(process.argv[1]), '..');
  const info = PLATFORMS[`${process.platform}-${process.arch}`];
  if (!info) return;

  const binDir = path.join(root, 'bin', info.dir);
  const binPath = path.join(binDir, info.exe);

  const ok = await ensureBinary(info, binDir, binPath);
  if (!ok) return;

  const child = spawn(binPath, ['--event', event], { stdio: 'ignore' });
  child.on('error', () => process.exit(0));
  child.on('exit', () => process.exit(0));
}

main().catch(() => process.exit(0));
