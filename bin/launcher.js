#!/usr/bin/env node
// roaring-crab cross-platform hook launcher.
//
// Invoked by hooks.json as:
//     node "${CLAUDE_PLUGIN_ROOT}/bin/launcher.js" <HookEvent>
//
// Detects the current platform + arch, locates the matching prebuilt
// `roaring-crab` binary under `${CLAUDE_PLUGIN_ROOT}/bin/<platform>/`,
// and execs it with `--event <HookEvent>`. Exits 0 silently when the
// platform isn't supported or the binary is missing, so hooks never
// surface errors to Claude Code.

const path = require('path');
const fs = require('fs');
const { spawn } = require('child_process');

const event = process.argv[2];
if (!event) process.exit(0);

const root =
  process.env.CLAUDE_PLUGIN_ROOT ||
  path.resolve(path.dirname(process.argv[1]), '..');

const map = {
  'win32-x64': 'windows-x86_64/roaring-crab.exe',
  'linux-x64': 'linux-x86_64/roaring-crab',
  'linux-arm64': 'linux-aarch64/roaring-crab',
  'darwin-x64': 'macos-x86_64/roaring-crab',
  'darwin-arm64': 'macos-aarch64/roaring-crab',
};

const rel = map[`${process.platform}-${process.arch}`];
if (!rel) process.exit(0);

const bin = path.join(root, 'bin', rel);
if (!fs.existsSync(bin)) process.exit(0);

const child = spawn(bin, ['--event', event], {
  stdio: 'ignore',
  detached: false,
});
child.on('error', () => process.exit(0));
child.on('exit', () => process.exit(0));
