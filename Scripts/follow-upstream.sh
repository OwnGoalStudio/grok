#!/usr/bin/env bash
#
# Pin this packaging repo to the public source commit matching the newest
# stable @xai-official/grok npm release. xai-org/grok-build intentionally has
# no release tags, so the crate's lockstepped version is the source-side join.
#
#   Scripts/follow-upstream.sh           # update Configuration/ if newer
#   Scripts/follow-upstream.sh --check   # exit 1 when a newer stable exists
#   Scripts/follow-upstream.sh --dry-run # print the candidate, change nothing
#   Scripts/follow-upstream.sh --print   # one line: version source-sha

set -Eeuo pipefail

mode=apply
case "${1:-}" in
"") ;;
--check) mode=check ;;
--dry-run) mode=dry-run ;;
--print) mode=print ;;
-h | --help)
    sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
*)
    echo "usage: $0 [--check|--dry-run|--print]" >&2
    exit 64
    ;;
esac

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
# shellcheck source=../Configuration/upstream.env
source "$repository_root/Configuration/upstream.env"

: "${UPSTREAM_REPO:?}"
: "${UPSTREAM_REF:?}"
: "${RUST_TOOLCHAIN:?}"

for tool in git npm python3; do
    command -v "$tool" >/dev/null || { echo "error: $tool is not installed" >&2; exit 69; }
done

current_version="$(tr -d '[:space:]' <"$repository_root/Configuration/version.txt")"
current_upstream_version="${current_version%%-*}"
upstream_version="$(npm view @xai-official/grok@latest version --json | python3 -c 'import json,sys; print(json.load(sys.stdin))')"
[[ "$upstream_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
    echo "error: npm latest is not a stable X.Y.Z version: $upstream_version" >&2
    exit 65
}

candidate_repo="$(mktemp -d "${TMPDIR:-/tmp}/grok-upstream.XXXXXX")"
trap 'rm -rf -- "$candidate_repo"' EXIT
git -C "$candidate_repo" init --quiet
git -C "$candidate_repo" remote add origin "$UPSTREAM_REPO"
git -C "$candidate_repo" fetch --quiet --depth 100 origin refs/heads/main

version_manifest="crates/codegen/xai-grok-version/Cargo.toml"
upstream_ref=""
while read -r candidate_ref; do
    candidate_version="$(
        git -C "$candidate_repo" show "$candidate_ref:$version_manifest" 2>/dev/null |
            awk '$0 == "[package]" { in_package = 1; next }
                 in_package && $1 == "version" { gsub(/"/, "", $3); print $3; exit }'
    )"
    if [[ "$candidate_version" == "$upstream_version" ]]; then
        upstream_ref="$candidate_ref"
        break
    fi
done < <(git -C "$candidate_repo" rev-list FETCH_HEAD)

[[ "$upstream_ref" =~ ^[0-9a-f]{40}$ ]] || {
    echo "error: no recent public source commit carries stable version $upstream_version" >&2
    exit 65
}

if [[ "$mode" == "print" ]]; then
    printf '%s %s\n' "$upstream_version" "$upstream_ref"
    exit 0
fi

echo "current:  $current_version @ $UPSTREAM_REF"
echo "upstream: $upstream_version @ $upstream_ref"

version_order="$(python3 - "$upstream_version" "$current_upstream_version" <<'PY'
import sys
candidate = tuple(map(int, sys.argv[1].split(".")))
current = tuple(map(int, sys.argv[2].split(".")))
print((candidate > current) - (candidate < current))
PY
)"
if (( version_order <= 0 )); then
    echo "no newer stable version than $current_upstream_version"
    exit 0
fi

if [[ "$mode" == "check" ]]; then
    echo "pin is behind stable $upstream_version"
    exit 1
fi

git -C "$candidate_repo" checkout --quiet --detach "$upstream_ref"
for patch in "$repository_root"/patches/*.patch; do
    git -C "$candidate_repo" apply --whitespace=nowarn "$patch" || {
        echo "error: $(basename "$patch") does not apply to stable $upstream_version ($upstream_ref)" >&2
        exit 65
    }
done

candidate_toolchain="$(python3 - "$candidate_repo/rust-toolchain.toml" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as source:
    print(tomllib.load(source)["toolchain"]["channel"])
PY
)"
[[ -n "$candidate_toolchain" ]] || {
    echo "error: stable source has no Rust toolchain channel" >&2
    exit 65
}

if [[ "$mode" == "dry-run" ]]; then
    echo "dry-run: would pin $upstream_ref, version $upstream_version, rustc $candidate_toolchain"
    exit 0
fi

python3 - "$repository_root/Configuration/upstream.env" "$upstream_ref" "$candidate_toolchain" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
replacements = {"UPSTREAM_REF": sys.argv[2], "RUST_TOOLCHAIN": sys.argv[3]}
seen = set()
lines = []
for line in path.read_text().splitlines(keepends=True):
    key = line.split("=", 1)[0]
    if key in replacements:
        lines.append(f"{key}={replacements[key]}\n")
        seen.add(key)
    else:
        lines.append(line)
missing = replacements.keys() - seen
if missing:
    raise SystemExit(f"{path} is missing: {', '.join(sorted(missing))}")
path.write_text("".join(lines))
PY
"$repository_root/Scripts/apply-version.sh" "$upstream_version"
make -C "$repository_root" source
echo "ready to tag v$upstream_version"
