#!/usr/bin/env bash
#
# Regenerates the libprussia-rt.a checked into this directory, which build.rs falls back to when
# it cannot assemble (host-only builds). build.rs assembles the same sources into OUT_DIR with the
# same flags, so this only needs running after a change to src/*.S - and it *does* need running
# then, because a stale archive is what an EE build silently links when its assembler is missing.
#
#   ./rebuild-asm-lib.sh
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)

# Flags come from asm.flags, which is what build.rs assembles with. One copy of the list means the
# checked-in archive and a from-source build cannot disagree about the ABI.
ASFLAGS=${ASFLAGS:-$(cat "$script_dir/asm.flags")}

# Prefer the ps2dev assembler (the one the EE link actually uses), then a generic mips64el.
if [ -z "${AS:-}" ]; then
    for candidate in mips64r5900el-ps2-elf-as mips64el-none-elf-as; do
        if command -v "$candidate" >/dev/null 2>&1; then
            AS=$candidate
            break
        fi
    done
fi
AS=${AS:-mips64el-none-elf-as}
AR=${AR:-${AS%as}ar}

cd "$script_dir"

# Sanity check the assembler. Read the whole --version output into a variable first: `grep -q`
# exits on the first match, and under pipefail the resulting SIGPIPE from the assembler would
# look like a failure.
version=$("$AS" --version 2>&1 || true)
case "$version" in
    *[Mm][Ii][Pp][Ss]*) ;;
    *)
        echo "$AS does not support MIPS (first line: $(printf '%s' "$version" | head -1))" >&2
        exit 1
        ;;
esac

# Assemble into a scratch directory so a failure cannot destroy a good archive.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

objects=()
for src in src/*.S; do
    name=$(basename "$src")
    echo "  AS $name"
    $AS $ASFLAGS -o "$tmp/$name.o" "$src"
    objects+=("$tmp/$name.o")
done

echo "  AR libprussia-rt.a"
rm -f libprussia-rt.a
$AR crs libprussia-rt.a "${objects[@]}"

"$AR" t libprussia-rt.a | sed 's/^/    /'
