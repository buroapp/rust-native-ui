#!/usr/bin/env bash
# Generate component documentation with a screenshot per component.
#
# Lazy by design: the catalog is the single source of truth. `storybook --list`
# dumps it as JSON; we loop, open each component with the existing `--select`
# deep link, and grab the window with the native `screencapture` — no screenshot
# library, no headless renderer, no second catalog to keep in sync.
#
# Usage: scripts/gen-docs.sh
set -euo pipefail
cd "$(dirname "$0")/.."

BIN=target/debug/storybook
OUT=docs/components
SHOTS=$OUT/img
WINID=target/winid

cargo build -p storybook --features macos >/dev/null 2>&1
# Native window-id helper, so we capture storybook's own window by CG id rather
# than a screen region — a region grab catches whatever overlaps it (a dialog, a
# stray window). No dependency: swiftc ships with macOS.
[ -x "$WINID" ] || swiftc -O scripts/winid.swift -o "$WINID"
mkdir -p "$SHOTS"

CATALOG=$($BIN --list)
COUNT=$(echo "$CATALOG" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
echo "Documenting $COUNT components"

# Capture one component by storybook's CG window id.
shoot() {  # $1 = index, $2 = output png
    "$BIN" --select "$1" >/dev/null 2>&1 &
    local pid=$!
    sleep 1.2  # let the window draw
    local wid
    wid=$("$WINID" storybook 2>/dev/null || true)
    if [ -n "$wid" ]; then
        screencapture -x -o -l"$wid" "$2" 2>/dev/null || true
        # Retina capture is ~2000px wide and ~2MB; docs don't need that. sips is
        # native macOS, so no image dependency. Cap width at 900px.
        [ -f "$2" ] && sips --resampleWidth 900 "$2" >/dev/null 2>&1 || true
    fi
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
}

# Generate per-component shots and collect rows grouped by category.
python3 - "$CATALOG" <<'PY' > /tmp/doc_plan.txt
import json, sys
cat = json.loads(sys.argv[1])
for e in cat:
    print(f"{e['index']}\t{e['category']}\t{e['name']}\t{e['summary']}")
PY

while IFS=$'\t' read -r idx category name summary; do
    png="$SHOTS/${name}.png"
    echo "  [$idx] $category / $name"
    shoot "$idx" "$png"
done < /tmp/doc_plan.txt

# Build the markdown from the same plan.
python3 - "$CATALOG" "$OUT" <<'PY'
import json, sys, os, collections
cat = json.loads(sys.argv[1])
out = sys.argv[2]
groups = collections.OrderedDict()
for e in cat:
    groups.setdefault(e["category"], []).append(e)

# Index page.
with open(os.path.join(out, "README.md"), "w") as f:
    f.write("# rn component catalog\n\n")
    f.write(f"{len(cat)} components, native AppKit controls on macOS. ")
    f.write("Each screenshot is the live component rendered by the `storybook` example.\n\n")
    f.write("Regenerate with `scripts/gen-docs.sh`.\n\n")
    for category, items in groups.items():
        f.write(f"## {category}\n\n")
        f.write("| Component | What it is |\n|---|---|\n")
        for e in items:
            anchor = e["name"].lower()
            f.write(f"| [{e['name']}](#{anchor}) | {e['summary']} |\n")
        f.write("\n")
    # Component sections with screenshots.
    f.write("---\n\n")
    for category, items in groups.items():
        for e in items:
            name = e["name"]
            f.write(f"### {name}\n\n")
            f.write(f"*{e['category']}* — {e['summary']}\n\n")
            img = f"img/{name}.png"
            if os.path.exists(os.path.join(out, img)):
                f.write(f"![{name}]({img})\n\n")
            else:
                f.write("_screenshot unavailable_\n\n")
            f.write(f"Open live: `cargo run -p storybook --features macos -- --select {name}`\n\n")
print(f"wrote {out}/README.md")
PY

echo "Done. See $OUT/README.md"
