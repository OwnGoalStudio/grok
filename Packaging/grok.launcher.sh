#!@PREFIX@/bin/sh
#
# The real binary lives in libexec so a later sidecar (resource files, a
# bundled ripgrep, a Swift back-deployment dylib) can sit beside it without
# polluting /usr/bin. A symlink from /usr/bin would also work today — grok
# does not resolve a SwiftPM bundle against argv[0] — but the launcher keeps
# the install layout identical to kk and makes the prefix substitution for
# the interpreter line a single code path.
#
# The package prefix is substituted at package time: empty for roothide, where
# this launcher is interpreted by the bootstrap's vroot-linked shell, and
# /var/jb for rootless, where every path has to be spelled out — including
# this script's own interpreter. The Rust process itself is not vroot-linked.
exec @PREFIX@/usr/libexec/grok/grok "$@"
