# roaring-crab — design

A Claude Code plugin, written in Rust, that plays generated analog-modeling-synthesis sounds on Claude Code hook events. Cross-platform (Windows, macOS, Linux). Sounds are abstract and musical — themed families with per-event variation. A long-lived daemon owns the audio device; short-lived client invocations dispatched by each hook send fire-and-forget play requests.

## Goals

- Ambient sonic feedback while working in Claude Code, with louder/melodic alerts for attention-required events.
- All 9 standard Claude Code hooks supported; per-hook on/off and master volume in user config.
- Sounds are synthesized at runtime via analog-modeling DSP (no audio files). Each hook has a signature patch with seeded variation so it feels alive rather than robotic.
- Hooks return in ~10ms so Claude Code never feels lagged by the plugin.
- Overlapping events layer naturally rather than queueing.
- Plugin installs cleanly with no build toolchain required on the user's machine.

## Non-goals

- No bundled or user-provided audio files.
- No per-hook patch overrides or DSP parameter tweaking in v1 (config exposes only volume, mute, per-hook enable).
- No tool-aware variation (e.g., different PreToolUse sound per tool) in v1.
- No bit-exact audio snapshot testing.
- No JACK / pro-audio backend on Linux beyond what cpal's default backend provides.

## Architecture

Two processes built from the same crate:

- **`roaring-crab` (client)** — short-lived. Invoked by each Claude Code hook. Sends a single `PlayEvent` to the daemon over a local socket and exits in ~10–15ms.
- **`roaring-crabd` (daemon)** — long-lived but idle-exiting. Owns the cpal audio output stream. Maintains an active-voice mixer. Lazy-spawned by the client on first event; exits after 5 minutes of no events and no active voices.

```
Claude Code hook fires
        │
        ▼
roaring-crab --event Stop  ──(local socket)──►  roaring-crabd
   (exits immediately)                          ├─ voice mixer ──► cpal ──► speakers
                                                ├─ idle timer
                                                └─ exits after 5 min idle
```

IPC: `interprocess` crate for cross-platform local sockets — Unix domain socket on macOS/Linux at `$XDG_RUNTIME_DIR/roaring-crab.sock` (or `$TMPDIR` fallback), named pipe on Windows at `\\.\pipe\roaring-crab-<username>`.

Audio: `cpal` for cross-platform output (WASAPI on Windows, CoreAudio on macOS, ALSA on Linux). 48kHz stereo f32.

Synthesis: `fundsp` for the DSP graphs. Each patch is a small `fundsp` expression combining oscillators, filters, ADSR envelopes, and effects.

## Components

### Shared library (`src/lib.rs`)

- **`protocol`** — wire format. One message type: `PlayEvent { event: HookEvent, seed: u64, volume: f32 }`. Encoded with bincode, length-prefixed framing (u32 BE length + payload). Strict max frame size (e.g., 1 KiB) on the daemon side.
- **`config`** — TOML config at the platform-appropriate user config dir, resolved via the `directories` crate:
  - Linux: `$XDG_CONFIG_HOME/roaring-crab/config.toml` (defaults to `~/.config/roaring-crab/config.toml`)
  - macOS: `~/Library/Application Support/roaring-crab/config.toml`
  - Windows: `%APPDATA%\roaring-crab\config.toml`
  
  Schema:
  ```toml
  master_volume = 0.7    # 0.0 – 1.0
  muted = false
  
  [enabled_hooks]
  SessionStart    = true
  SessionEnd      = true
  UserPromptSubmit = true
  PreToolUse      = true
  PostToolUse     = true
  Notification    = true
  Stop            = true
  SubagentStop    = true
  PreCompact      = true
  ```
  
  Missing file → defaults are written to disk on first run. Unknown fields are tolerated for forward compatibility. Malformed TOML produces a `ConfigError` naming the offending line; the client logs and continues with defaults.
- **`hook_event`** — `enum HookEvent` with the 9 variants: `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Notification`, `Stop`, `SubagentStop`, `PreCompact`. Derives `Serialize`, `Deserialize`, `clap::ValueEnum`, `Copy`, `Eq`, `Hash`.
- **`patches`** — one submodule per hook event. Each exposes `fn build(seed: u64) -> An<impl AudioUnit64>` returning a `fundsp` graph plus a `pub const DURATION_MS: u32` declaring how long the voice should live. Patches are organized into three families:
  - **Ambient** — `PreToolUse`, `PostToolUse`, `UserPromptSubmit`. Short filtered blips, 100–200ms, low headroom so they sit in the background.
  - **Lifecycle** — `SessionStart`, `SessionEnd`, `PreCompact`. Warm pad-ups, descending sweeps, longer envelopes, 600–1000ms.
  - **Alert** — `Notification`, `Stop`, `SubagentStop`. Melodic motifs — rising arpeggio, resolved chord, short two-note resolution. 400–800ms.
  
  Seeded variation: each invocation seeds an internal RNG that perturbs detune cents, filter cutoff, envelope timing, and voicing selection within a per-patch range. Same hook always sounds *like itself*; never sounds *identical* twice in a row.
- **`mixer`** — `struct Mixer { voices: Mutex<Vec<Voice>>, master_volume: AtomicF32 }`. `Voice` wraps a `fundsp` graph plus `samples_remaining: u32`. The mixer is `Send + Sync` and shared between the accept thread and the cpal callback thread.

### Client binary (`src/bin/roaring-crab.rs`)

1. Parse CLI: `roaring-crab --event <HookEvent>` via clap. Unknown event → exit 2 with stderr message (config bug, loud failure).
2. Drain stdin (Claude pipes a JSON event payload; we ignore its contents in v1 but must drain).
3. Load config. If `muted || !enabled_hooks[event]` → exit 0.
4. Connect to the local socket. On `ConnectionRefused`/`NotFound`:
   - Spawn `roaring-crabd` detached. On Windows: `CREATE_NO_WINDOW | DETACHED_PROCESS` so no console flashes. On Unix: `setsid` / `fork` so the daemon is reparented to init.
   - Poll the socket up to 200ms (10ms intervals). If still not responsive, log to stderr and exit 0.
5. Serialize `PlayEvent { event, seed: rand::random(), volume: config.master_volume }`. Write length-prefixed frame. Close. Exit 0.

The client never returns nonzero for runtime audio failures — Claude Code should always see the hook succeed.

### Daemon binary (`src/bin/roaring-crabd.rs`)

1. Acquire a per-user lockfile (`<runtime_dir>/roaring-crab.lock`). If lock is held but the holding PID isn't alive, reclaim it (stale lock). If a live daemon owns it, exit 0 silently.
2. Open the cpal default output stream (48kHz stereo f32). On failure (no audio device, headless server), log to the daemon log file and exit; release the lock so future events can retry.
3. Bind the local socket. Spawn an accept thread.
4. Audio callback (cpal-owned thread): for each output buffer, lock the voice vec briefly, sum all active voices' next samples with `master_volume` applied, write to the output buffer. Decrement each voice's `samples_remaining`; remove voices that hit zero. Clamp final output to `[-1.0, 1.0]` to prevent runaway samples.
5. Accept loop: for each incoming connection, read a length-prefixed bincode frame, deserialize `PlayEvent`, build the patch, wrap in `Voice`, push onto the mixer. Reject frames over the size cap.
6. Voice cap: hard limit of 16 active voices. When pushing a 17th, drop the oldest first.
7. Idle watchdog thread: ticks every second. When `voices.is_empty() && time_since_last_event > 5min`, close the cpal stream cleanly and exit.

## Configuration

User-visible knobs only, in `config.toml`:

- `master_volume`: f32 in `[0.0, 1.0]`. Applied at the mixer output.
- `muted`: bool. When true, client exits before connecting to the daemon. No sound produced.
- `enabled_hooks`: map of `HookEvent` → bool. When a hook is disabled, the client exits before connecting.

Defaults: `master_volume = 0.7`, `muted = false`, all hooks enabled.

The config is read fresh on every client invocation (zero-state client design — no caching, no daemon reload). The daemon does not read the config at all; volume is passed per-event in the `PlayEvent` message.

## Distribution

The plugin repo ships prebuilt binaries under `bin/<platform>/`:

```
bin/
  linux-x86_64/{roaring-crab, roaring-crabd}
  linux-aarch64/{roaring-crab, roaring-crabd}
  macos-x86_64/{roaring-crab, roaring-crabd}
  macos-aarch64/{roaring-crab, roaring-crabd}
  windows-x86_64/{roaring-crab.exe, roaring-crabd.exe}
```

The plugin's `hooks.json` wires every Claude Code hook to a small platform-detecting launcher script:

- `bin/launch.sh` for Unix (one script chooses between `linux-*` and `macos-*` by `uname`).
- `bin/launch.cmd` for Windows.

`hooks.json` uses two parallel entries per hook (Unix shell vs. Windows cmd) so the same plugin works regardless of OS.

CI (GitHub Actions): matrix build across the five target triples on every push; on tag, build release binaries and commit them into `bin/<platform>/` via a release workflow, then publish a GitHub release. Binaries are ad-hoc-signed on macOS to minimize Gatekeeper friction. Users without code-signing acceptance may need to manually approve on first run; document this in the README.

## Data flow (single event)

1. Claude Code fires (e.g.) the `Stop` hook. `hooks.json` matches it and runs the launcher with `--event Stop`.
2. Launcher execs the platform-correct `roaring-crab` binary. Client starts (~10ms), parses args, drains stdin.
3. Client loads config. If `muted || !enabled_hooks[Stop]` → exit 0.
4. Client opens local socket. If unavailable: spawn `roaring-crabd` detached, poll up to 200ms, retry.
5. Client serializes `PlayEvent { event: Stop, seed: rand::random(), volume: master_volume }`, writes length-prefixed frame, closes. Exits.
6. Daemon accept thread reads frame, deserializes, calls `patches::stop::build(seed)`, wraps in `Voice { graph, samples_remaining }`, pushes onto mixer.
7. cpal audio callback (~10ms intervals): locks voice vec, sums each voice's next samples × master_volume into the output buffer. Voices whose `samples_remaining` hits zero are removed.
8. Idle watchdog: every second, if no voices and no `PlayEvent` for 5 min, daemon closes the cpal stream and exits.

**Concurrent events:** two hooks firing back-to-back produce two `PlayEvent`s; both voices live in the mixer and play simultaneously. Overlapping textures are part of the aesthetic.

## Error handling

Guiding principle: **a notification plugin must never make Claude Code's hooks feel broken.** Sound failures are silent; the hook still appears to succeed.

### Client

- Config file unreadable or malformed → write defaults if missing, log a one-line warning to stderr, continue with defaults.
- Socket connect fails AND daemon spawn fails (e.g., binary missing for platform) → log, exit 0.
- Daemon spawned but socket doesn't appear within 200ms → log "daemon slow to start, skipping event", exit 0.
- Stdin read error → ignore.
- Unknown `--event` value → exit 2 with stderr message. This is the only nonzero exit path.

### Daemon

- cpal device unavailable → log to `<config_dir>/roaring-crabd.log`, release lockfile, exit. Future events retry-spawn.
- Stale lockfile (holding PID dead) → reclaim and continue.
- Socket bind fails → log and exit. Future events keep retry-spawning harmlessly.
- Audio callback panic inside a voice → `std::panic::catch_unwind` per voice; drop the voice, keep the mixer running.
- Patch produces NaN/inf samples → mixer clamps output to `[-1.0, 1.0]` before cpal, so the worst case is a clipped sample, not a speaker-blasting DC offset.

### Logging

- Client: stderr only. Claude shows hook stderr on hook errors.
- Daemon: `<config_dir>/roaring-crabd.log`, capped at ~1 MiB with one rolled-over `.log.old`.
- Identical warnings rate-limited to once per minute.

### Cross-platform gotchas

- **Windows**: detached daemon spawn must set `CREATE_NO_WINDOW | DETACHED_PROCESS` (no console flash).
- **macOS**: unsigned binaries may trigger Gatekeeper prompts. Binaries ad-hoc-signed in CI; document approval steps in README.
- **Linux**: cpal default backend is ALSA; PipeWire/PulseAudio work via ALSA emulation. Document this; do not add a JACK fallback.

## Testing

### Unit tests (`cargo test`)

- **`protocol`**: round-trip every `HookEvent` through bincode; length-prefix framing parses correctly with trailing garbage; oversized frames are rejected.
- **`config`**: defaults written when file missing; malformed TOML produces `ConfigError` with line info; unknown fields tolerated; per-hook enable map deserializes correctly.
- **`hook_event`**: clap parses every variant; serde round-trips; unknown event strings produce clean errors.
- **`patches`**: for each patch module, render 1 second to an in-memory buffer with three different seeds. Assert: no NaN/inf, peak amplitude ≤ 1.0, RMS above a small silence threshold, two different seeds produce measurably different output (the variation contract).
- **`mixer`**: voice push is thread-safe under stress; voice cap of 16 enforced; finished voices removed; sum of N silent voices is silent.

### Integration tests (`tests/`)

- **`client_to_daemon.rs`**: spawn the daemon with a temp socket path env var and a `--null-audio` flag (no cpal device), connect from a test client, send a `PlayEvent`, assert the voice was added to the mixer. Tear down.
- **`lazy_spawn.rs`**: no daemon running; invoke the client; assert daemon process exists, socket responsive; set `RC_IDLE_SECS=1`; assert daemon exits cleanly.
- **`concurrent_events.rs`**: fire 20 client invocations in parallel; assert daemon receives all 20 (or exactly 16 voices remain if cap kicks in), no crashes, no deadlock.

### Manual / acceptance

- `examples/audition.rs`: walks through all 9 patches × 3 seeds with real audio out for sound design iteration. Not in CI.
- README "How to verify install" section: one command fires one patch.

### CI

- GitHub Actions matrix: `{ubuntu-latest, macos-latest, windows-latest}` plus aarch64 cross-compilation where applicable. Build, `cargo test --features null-audio` (so cpal isn't required on headless runners).
- On tag: build release binaries for all five targets, commit them into `bin/<platform>/`, publish a GitHub release.

### Out of scope for v1

- Snapshot tests on audio output (not bit-exact across cpal backends / sample rates).
- Performance benchmarks (latency budget is generous; no need to gate on it).

## Open questions

None at design time. Patch sound design is intentionally underspecified — it will be iterated on with `examples/audition.rs` during implementation.
