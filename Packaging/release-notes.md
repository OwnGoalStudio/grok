[Grok Build](https://github.com/xai-org/grok-build) — SpaceXAI's terminal coding agent — built for jailbroken iOS and installed as `grok`.

## Which one do I download?

The architecture field names the **bootstrap layout, not the CPU**. Both packages carry the same arm64 binary.

| your bootstrap | download |
| -------------- | -------- |
| rootless (Dopamine, palera1n rootless) | `@PACKAGE_ID@_@VERSION@_iphoneos-arm64.deb` |
| roothide (RootHide Dopamine) | `@PACKAGE_ID@_@VERSION@_iphoneos-arm64e.deb` |

Not sure? Ask the device: `dpkg --print-architecture`.

Requires **iOS @MIN_IOS_MAJOR@ or later** and a bootstrap that provides a shell. Or add the [OwnGoal Studio repository](https://github.com/OwnGoalStudio/OwnGoalPackages) and let your package manager pick.

## Usage

Run `grok` in a terminal on device. Authenticate with an xAI API key or the browser login (`uiopen` opens the URL).

## About this build

Upstream [`xai-org/grok-build@@UPSTREAM_SHORT@`](https://github.com/xai-org/grok-build/commit/@UPSTREAM_REF@), plus the patches that port it to a jailbroken iOS userspace. The binary is a Rust executable: it is **not** linked with libvroot, so it has to probe for the bootstrap's shell instead of trusting `/bin/sh`. See [`patches/`](https://github.com/OwnGoalStudio/grok/tree/@TAG@/patches).

Verify your download against `SHA256SUMS`.

**Full changelog**: https://github.com/OwnGoalStudio/grok/commits/@TAG@
