# Quickstart — Validating Platform Parity

**Feature**: `003-platform-parity`

How to prove the feature works, on each platform, from a clean checkout. Each section states the command and the outcome that counts as success. If an outcome does not match, the feature is not done regardless of what the tests say.

---

## Prerequisites

| Platform | Requirement |
|---|---|
| All | Rust 1.75+ with `cargo` |
| Windows | Windows 10 22H2 or later; Visual Studio Build Tools (MSVC target) |
| Linux | GTK 4.8+ development files (`libgtk-4-dev` or equivalent); `xvfb` for headless verification |
| macOS | Xcode command line tools (reference platform, unchanged by this feature) |

Verify GTK is discoverable before anything else:

```sh
pkg-config --modversion gtk4        # must print 4.8 or higher
```

If this fails, the Linux backend cannot build and no further validation is meaningful.

---

## 1. The build compiles on all three platforms

```sh
# macOS
cargo build --workspace --features rn/macos

# Windows
cargo build --workspace --features rn/windows

# Linux
cargo build --workspace --features rn/linux
```

**Success**: all three exit zero with no warnings on the `rn-windows` and `rn-linux` crates.

**The real assertion is negative**: no application source under `examples/` was edited to make this pass. `git diff --stat examples/` must be empty for this feature's branch.

---

## 2. Completeness is enforced, not assumed

```sh
cargo test -p rn-windows --features windows capabilities
cargo test -p rn-linux   --features linux   capabilities
```

**Success**: each prints a count equal to the number of `ComponentKind` variants, and the test compares that count against the enum rather than a hard-coded number.

**Prove the guard actually works** — this is the check people skip:

1. Add a throwaway variant to `ComponentKind` in `rn-core/src/lib.rs`.
2. Run `cargo build -p rn-windows --features windows` and `cargo build -p rn-linux --features linux`.
3. Both MUST fail with a non-exhaustive match error naming the new variant.
4. Remove the variant; both build again.

If step 3 does not fail, FR-003 is not implemented, whatever the test output says.

---

## 3. The applications actually run

Run each example on Windows and on Linux:

```sh
cargo run -p counter        --features rn/windows     # then: --features rn/linux
cargo run -p gallery        --features rn/windows
cargo run -p showcase       --features rn/windows
cargo run -p settings-demo  --features rn/windows
cargo run -p mail-demo      --features rn/windows
```

**Success** per application:

| Application | What must be visibly true |
|---|---|
| `counter` | Number increments on click; the label updates |
| `gallery` | Every control is drawn by the OS, not by the framework |
| `showcase` | All sections render; nothing is a blank rectangle |
| `settings-demo` | Sidebar navigates; switches toggle; rows show icons and chevrons |
| `mail-demo` | Three panes visible; folder selection changes the message list |

A blank window is the specific failure this feature exists to eliminate. Treat any empty region as a failure, not a cosmetic issue.

---

## 4. Layout agreement across platforms

Render the same tree at the same window size on all three, capture the computed bounds, and compare.

**Success**: every node's `x`, `y`, `width`, `height` agrees across platforms within **2 logical pixels** (SC-003).

Use the existing debug instrumentation on the macOS backend as the reference — it already logs computed rects. Run the other backends with the same logging enabled and diff the output for an identical tree.

Text-heavy nodes are expected to differ slightly in measured width because fonts differ per platform; the tolerance accounts for this. A node differing by tens of pixels indicates the backend is computing geometry, which C3 forbids.

---

## 5. Large list stays bounded

```sh
cargo run -p settings-demo --features rn/windows -- --bench-flatlist
cargo run -p settings-demo --features rn/linux   -- --bench-flatlist
```

**Success**: a ten-thousand-item list mounts **fewer than 100 native controls**, reported by the same live-node instrumentation macOS already uses (SC-006). The number must not grow as the user scrolls.

If it grows linearly with item count, virtualisation is not wired to the native list's own virtual mode (`LVS_OWNERDATA` on Windows, `GtkListView` on Linux) and the backend is materialising every row.

---

## 6. State updates, not just initial state

The failure mode this catches is subtle and common: a control that renders correctly on first paint and then ignores every subsequent change.

On each platform, with `settings-demo` running:

1. Toggle Wi-Fi off. The switch must move and the status text must change.
2. Change the appearance segment. The selection must move.
3. Type in the search field. Rows must filter.
4. Resize the window. Content must reflow, not clip.

**Success**: all four behave correctly on all three platforms. Any control that renders but does not respond to state is an FR-009 failure.

---

## 7. Accessibility

Inspect the platform accessibility tree of a running application.

- **Windows**: Inspect.exe or the Accessibility Insights tree view
- **Linux**: Accerciser, or `gtk4` inspector's a11y overlay
- **macOS**: the accessibility tree via the system inspector (reference)

**Success**: every interactive control exposes a non-empty role and name, and a control with a visible label uses that label as its name (SC-007). A tree full of unnamed buttons is a failure even though the UI looks correct.

Then, keyboard only, without touching the mouse:

**Success**: Tab moves focus through every interactive control in tree order on all three platforms.

---

## 8. Display protocols (Linux)

```sh
# X11
GDK_BACKEND=x11 cargo run -p settings-demo --features rn/linux

# Wayland
GDK_BACKEND=wayland cargo run -p settings-demo --features rn/linux
```

**Success**: both open, both accept input, both close cleanly (SC-008). Window decorations will differ between the two; that is expected and correct.

Headless verification, for CI:

```sh
xvfb-run -a cargo test -p rn-linux --features linux
```

**Success**: windowed tests run rather than skip. If they skip, the display detection is wrong — check that `DISPLAY` is set by `xvfb-run`.

---

## 9. Scaling

**Windows**: move the window between a 100%-scale and a 150%-scale monitor.

**Success**: text stays sharp at both, and the layout does not clip or overflow after the move. Blurry text means the process is not per-monitor DPI aware.

**Linux**: run with `GDK_SCALE=2`.

**Success**: everything scales uniformly; logical layout values are unchanged.

---

## Clean verification checklist

- [ ] All three platforms build the workspace with their feature flag
- [ ] `examples/` has zero diff on this branch (SC-005)
- [ ] Capability count equals the `ComponentKind` variant count on both new backends
- [ ] Adding a throwaway variant breaks both backends' builds (the guard is real)
- [ ] All five example applications run and are non-blank on both new platforms (SC-001)
- [ ] Node bounds agree within 2 logical pixels across platforms (SC-003)
- [ ] Ten-thousand-item list holds under 100 native controls on both new backends (SC-006)
- [ ] Toggle, segment, search, and resize all update live on both new backends
- [ ] Every interactive control has an accessible role and name (SC-007)
- [ ] Tab traversal reaches every interactive control
- [ ] Both Wayland and X11 work on Linux (SC-008)
- [ ] Text is sharp at 150% on Windows; uniform at `GDK_SCALE=2` on Linux
