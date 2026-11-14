# OBS-Studio-Rust — personal fork (ziyyhhk) — FINAL

> **Status: discontinued. v1.1.0 is the final update.** This repository is an
> unofficial, personal fork of [obs-rust/obs-studio](https://github.com/obs-rust/obs-studio)
> (the independent Rust rewrite of [OBS Studio](https://obsproject.com/)).
> It is **not** affiliated with the OBS Project, obsproject.com, or the
> obs-rust organization. Nobody is maintaining this; issues and forks are
> unmonitored. If you want ongoing development, use an official release
> instead.

## What this is

A copy of `obs-rust/obs-studio` master with three small, verified changes by
the fork owner, built for personal use. It is a beta of a beta: roughly half
of libobs is Rust, the rest is the upstream C/C++ hybrid build. It keeps the
upstream app identity ("OBS-Studio-Rust"), settings folder and plug-in
compatibility, and installs **beside** official OBS Studio.

## What the fork changed (all bit-exact against the C oracle)

1. **SSE2 block kernels for the UYVX packed-YUV packers**
   (`compress_uyvx_to_i420/nv12`, `convert_uyvx_to_i444`, in
   `rust/obs-media-io/src/platform/uyvx.rs`) — ~2.3× faster on that kernel.
   Helps packed UYVY/YUY2 capture devices; native NV12 sources are unaffected.
2. **AVX2 runtime dispatch for the sample_peak volmeter kernel** in
   `rust/obs-audio-controls` — mirrors the upstream `true_peak` dispatch
   (#3415). ~1–3% on the audio meter path.
3. **Release tooling**: a `release.yml` workflow that builds the Windows
   portable ZIP and BETA MSI on GitHub-hosted runners and attaches them to a
   release for the tag you dispatch with.

Both kernels are validated with the repo's parity tests (bit-for-bit against
the unmodified C implementation, including NaN behaviour).

## What "optimized" means here (and what it doesn't)

- **Recording/streaming CPU is dominated by the encoder** (NVENC/x264) and the
  browser source; that code is shared C/C++ with upstream and is unchanged.
- **RAM** benefit is inherited from upstream (e.g. ~19% lower launch memory).
  The fork adds no further RAM reduction.
- The meter and packer changes matter only where they run: audio ticks,
  meters, and packed-YUV camera feeds.

## Install (Windows, this fork, unsigned)

- **MSI** — run `OBS-Studio-Rust-<version>-windows-x64.msi`. Branded
  "OBS-Studio-Rust BETA", installs beside official OBS Studio. Unsigned, so
  Windows SmartScreen will warn; choose **More info → Run anyway**. Or
  right-click the file → Properties → Unblock first to silence it.
- **ZIP** — unpack and start `bin\64bit\obs64.exe`.
- First launch copies your existing OBS settings, profiles and scenes into
  `%APPDATA%\obs-studio-rust` without changing the originals. Later upstream
  changes are not imported again.

Other platforms and the signed upstream builds:
https://github.com/obs-rust/obs-studio/releases (use those instead).

## Finding the sound meters ("voice meter")

The fork ships the **unmodified upstream UI**, including the **Audio Mixer**
dock (per-source dB meters with peak readout, mute buttons, monitoring).
If it is not visible:

- **View → Docks → Audio Mixer**, or
- **View → Reset UI** to restore the default dock layout.

To check your microphone/speakers: open the dock, speak, and watch the
**Mic/Aux** bar; play sound and watch **Desktop Audio**. If a bar is flat,
that's a capture-settings problem, not a missing feature.

## Recording tips

- Prefer **NVENC** (Settings → Output) if you have an NVIDIA GPU; x264 is the
  fallback. Neither is fork-specific.
- The app logs to `%APPDATA%\obs-studio-rust\basic\logs` — maxima useful when
  a capture path misbehaves.

## Building it yourself

See the GitHub Actions workflow `.github/workflows/release.yml`. Dispatch:

```bash
gh workflow run release.yml --ref master -f tag=v1.1.0
```

It builds on the `windows-2025-vs2026` runner (ccache, soldr, WiX MSI
packaging — same setup as upstream's nightly), publishes the ZIP and MSI to
`https://github.com/ziyyhhk/obs-studio/releases` for the given tag, and costs
you zero disk locally.

## License

GPL-2.0-or-later, like upstream. This is a derived work of
https://github.com/obs-rust/obs-studio (itself a fork of
https://github.com/obsproject/obs-studio). "OBS" and OBS Studio are
trademarks of their respective owners; this project is not endorsed by them.

## Credits

Upstream authors, the obs-rust contributors (human and AI-assisted), and the
OBS Project whose code is the base of all of this. Please report bugs to a
fork, or just close this and use the official builds — the project is
discontinued.
