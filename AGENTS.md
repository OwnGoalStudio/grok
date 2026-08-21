# grok — Agent Notes

[Grok Build](https://github.com/xai-org/grok-build) — SpaceXAI's terminal coding
agent — built for jailbroken iOS 15+ and installed as `grok`, for both
**roothide** and **rootless** bootstraps.

This repository holds **no application source**. It fetches grok-build at a
pinned commit, applies `patches/`, cross-compiles for `aarch64-apple-ios`, and
packages. Everything runs through `Scripts/`, so CI and a local checkout
execute the same code.

## Hard rules

- **Not a fork.** Never vendor grok-build source here. Every change to it is a
  patch in `patches/`, applied by `Scripts/prepare-source.sh` to a fresh
  checkout of `UPSTREAM_REF`. Keep patches small and single-purpose.
- **`UPSTREAM_REF` is a full commit sha**, not a branch. Bump with
  `make bump-upstream REF=…`.
- **One arm64 binary backs both packages.** arm64 runs on every arm64e device
  and the reverse is not true. The two `.deb` architectures name a *bootstrap
  layout*, not a CPU: `iphoneos-arm64` is rootless, `iphoneos-arm64e` is
  roothide. Never build an arm64e slice for the "arm64e" package.
- **Never hardcode a bootstrap path in patched source.** Probe for the file and
  take the first that exists. Prefix substitution belongs in *packaging*
  (`@PREFIX@`), not in Rust.
- **Versions live in `Configuration/version.txt` only.** `X.Y.Z` tracks
  upstream's crate version; `X.Y.Z-N` is a packaging-only respin.
- **Do not link libvroot into this binary.** See below.

## libvroot is not our problem, and not our solution

RootHide's one-line summary is `roothide = rootful + libvroot`.

**libvroot** is a compile-time shim for C/C++ bootstrap programs. When Procursus
builds git, bash, ssh, it rewrites ~200 path APIs (`open`, `stat`, `execve`,
`posix_spawn`, …) so that `/bin/sh` and `/usr/bin/git` mean "those paths inside
the randomized jbroot", and the real iOS root is reached at `/rootfs/...`.
That is why a C program compiled into the bootstrap can keep writing `/bin/sh`
and still work on roothide, and why git's hardcoded `/bin/sh` for credential
helpers does **not** need a `/var/jb` patch on roothide.

Rootless has no such shim. `/bin/sh` is simply absent (`/var/jb/bin/sh` → dash).
That is the Procursus git `SHELL_PATH=$(MEMO_PREFIX).../bin/sh` patch.

This binary is **Rust**. rustc talks to libSystem directly. It is not built
with libvroot, and injecting vroot into a Rust std::process is not something
we will do. Consequences:

- A vroot parent shell may export `SHELL=/bin/zsh`. That path is true *inside*
  the parent, and a lie to this process: `is_executable("/bin/zsh")` looks at
  the real rootfs and fails on both bootstraps.
- Runtime lookup has to probe `/var/jb/bin` and `/var/jb/usr/bin` first. That
  hits rootless for real, and hits roothide through the compatibility symlink
  it keeps at `/var/jb`. Unprefixed `/bin` stays in the list for rootful and
  for a future vroot-linked world we are not in.
- **Packaging** is still two layouts. roothide dpkg unpacks an unprefixed tree
  into the jbroot it picked this boot; rootless dpkg unpacks under `/var/jb`.
  The architecture field (`iphoneos-arm64e` vs `iphoneos-arm64`) names that
  layout. Do not infer the layout from `/var/jb` existing — roothide has the
  symlink too. Ask `dpkg --print-architecture`.

Do not add `/var/jb` to patched source as "the rootless prefix". It is one
candidate in a probe list.

## Layout

```
Configuration/upstream.env   pinned ref, cargo package/bin, iOS floor, rustc
Configuration/version.txt    package version
patches/NNNN-*.patch         applied in sorted order to a pristine checkout
Packaging/DEBIAN/control     control template (@PLACEHOLDER@ substituted)
Packaging/grok.entitlements  what the signed binary carries, and why
Packaging/grok.launcher.sh   /usr/bin/grok → the real binary in libexec
Scripts/prepare-source.sh    fetch + patch (idempotent, stamped)
Scripts/build-ios.sh         cargo --target aarch64-apple-ios, verify Mach-O
Scripts/package-deb.sh       stage + ldid + dpkg-deb + verify
Scripts/install-device.sh    install over SSH and smoke-test (dev only)
build/                       everything generated; not source
vendor/nono/                 pinned nono 0.53.0 with the unsupported-OS dedup key fix
```

## Build & verify

- `make check` — script syntax, config sanity, patch set, packaging inputs
- `make source` — fetch + patch; fails loudly if a patch no longer applies
- `make build` — cross-compile and verify the Mach-O is iOS
- `make debs` — both packages plus `SHA256SUMS`; what CI releases
- Release runners must install DotSlash before `make debs`; the pinned
  grok-build tree's `bin/protoc` is a DotSlash launcher.
- Keep nono pinned at 0.53.0 and routed through `vendor/nono`: its upstream
  macOS/Linux-only dedup key does not compile for iOS, while the unsupported
  iOS sandbox path still needs the crate to build.
- iOS release builds must skip upstream's target-specific ripgrep bundle and
  depend on Procursus `ripgrep`; both consumers already fall back to `rg` on
  `PATH`, and ripgrep publishes no iOS release asset for the build scripts.
- Treat iOS as an Apple home-directory platform in workspace classification;
  otherwise the unconditional classifier call has no target implementation.
- Keep `pprof` and its Unix CPU-profiler implementation out of iOS builds;
  pprof 0.15 has no iOS backend, so the existing unsupported-runtime path is
  the truthful platform contract.
- Compile `xai-grok-voice` without its `audio` feature on iOS: upstream has no
  iOS capture backend, and the no-audio feature already exposes capture as
  unsupported without inventing a platform implementation.
- On iOS, skip `dark-light` desktop detection and continue through the existing
  environment/OSC 11 chain; dark-light's unsupported-target API has a different
  return type and no meaningful iOS system appearance result.
- `make install` — install on an attached device and run `--version`.
  Over USB: `iproxy 4422:2222 &`

Test by installing, never by copying a binary onto `/var/mobile`. A copied
binary runs with its entitlements ignored (trustcache never saw it).

## The OwnGoalPackages contract

Same as kk: a non-draft, non-prerelease tag `vX.Y.Z`; assets whose names end
in `iphoneos-arm64.deb` / `iphoneos-arm64e.deb`; a `SHA256SUMS` of bare names.
