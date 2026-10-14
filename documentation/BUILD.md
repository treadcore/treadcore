# How the build is designed

treadcore is developed in a 6 GB, 4-core Alpine VM under QEMU. Bevy is
a few hundred crates, and a naive `cargo build` of it on that machine
ran out of memory on `bevy_ui`. This document is what was learned and
what the build does about it.

## The problem, measured

- Bevy 0.17 with default features pulls **433 crates**; with only the
  features a 2D sprite/UI/gizmo window needs, **368**.
- A single `rustc` on one of the big crates (`bevy_ui`, `bevy_render`,
  `wgpu-core`, `naga`) wants **1 to 1.5 GB** at opt-level 3 with debug
  info. Cargo runs one per CPU by default: four at once is 4 to 6 GB,
  which on a 6 GB machine is a swap storm. Swapping rustc is slower
  than one rustc.
- Debug info for dependencies is the bulk of `target/`: the cancelled
  half-build was 3.7 GB.
- A release link with thin LTO over the whole Bevy graph is the single
  largest memory spike of the build.

## The design

### 1. One Bevy, defined once

`[workspace.dependencies.bevy]` at the root carries the version, the
feature list, and `default-features = false`. The window and the warm
crate both say `bevy = { workspace = true }`. Cargo caches artifacts by
(crate, version, features, profile, target), so two crates that resolve
Bevy identically share every artifact. That is the whole mechanism
behind `make warm`.

The feature list is the 2D subset: window, winit on Wayland and X11,
render, core pipeline, sprite, text with the default font, UI, gizmos,
state, asset, log, and audio with the WAV decoder (roadmap step 9:
every sound is synthesised in the window and handed to Bevy as WAV, so
no other format is needed). Not listed, and therefore not compiled:
PBR, glTF, the other audio formats, animation, picking, scenes, and
their trees.

### 2. `make warm`: the engine first, gently

`source/treadcore-warm` is a crate that depends on Bevy and draws one
sprite, one text node and one gizmo. `make warm` builds it, in dev and
release, with:

- the job count from `make configure` (see 3),
- `nice -n 15` and `ionice -c 3`, so the machine stays usable,
- nothing of the game in the graph, so a game compile error cannot
  interrupt a two-hour engine build.

Cargo resumes per crate, so ctrl-c costs only the crate in progress.
When it finishes, `make build` and `make run` compile treadcore's own
six crates and link; the engine is found, not rebuilt. The binary it
leaves, `target/release/treadcore-warm`, is also the test for "does a
window open on this machine at all", worth running before the game.

### 3. Jobs sized from memory, not CPUs

`scripts/configure.sh` computes

    JOBS = min(CPUs, (RAM_MB - 1024) / 1536), at least 1

and writes it to `build/config.env` as `CARGO_BUILD_JOBS`, which every
script exports. On this VM that is 3. `JOBS=n make warm` overrides.
Configure also warns when swap is under a gigabyte: without swap the
kernel kills a rustc outright instead of slowing it.

### 4. Profiles that fit

| profile | game crates | dependencies | LTO | used by |
| --- | --- | --- | --- | --- |
| dev | opt 1, debug info | opt 1, **no debug info** | off | `make debug`, `make test` |
| release | opt 3 | opt 3 | **off**, 16 codegen units | `make build`, `make run` |
| dist | opt 3 | opt 3 | thin, 1 codegen unit | by hand, for a binary to ship |

Dependency debug info is gigabytes that nobody steps into. LTO is
left for `dist` because the link is the memory spike and the gain is a
few percent on a game that is not CPU-bound.

### 5. Headless never waits for the engine

`sim`, `proto`, `net`, `brain` and `server` do not depend on Bevy.
`make debug`, `make test`, `make headless`, `make host`, `make seat`
and `make lobby-test` build and run with the engine entirely absent.
Most development is there.

### 6. One cargo at a time

Two cargo invocations in one workspace share `target/` and one takes a
lock while the other waits (it prints "Blocking waiting for file lock").
Running a test build beside a warm build does not parallelise anything;
it serialises it in a confusing order. The scripts do not run cargo in
the background for that reason.

## What to do on this machine

```
make configure          # see the jobs number and the swap warning
make warm               # once; go and do something else
make test               # headless crates, minutes
make debug              # a traced bot match
make run                # the window; compiles only the game now
```

If `make warm` still swaps at 3 jobs: `JOBS=2 make warm`. If the VM
has no swap, give it 2 GB; QEMU memory is cheaper than a killed link.

## The machine around the build

Measured on the development VM on 12 October 2026, with the build
stopped:

| what | RAM | note |
| --- | --- | --- |
| rust-analyzer (editor) | 928 MB resident + 2 GB swapped | the only large consumer; kill it during a warm |
| this agent's session | ~430 MB | |
| Chromium | ~100 MB | close it if tight |
| Hyprland | 48 MB | not a memory problem |
| zram swap, 3.1 GB device, lz4 | 943 MB stored in 620 MB of RAM | compresses only 1.5x |
| /tmp (tmpfs, in RAM) | 550 MB | leftovers from other sessions; deletable |

Hyprland's cost is CPU, not RAM: on a VM with no GPU (virtio, llvmpipe)
blur, shadows and animations are drawn by the CPU the compiler wants.
Idle, it was using 64% of a core; with effects off, 45%. `make quiet`
turns them off at runtime, `make loud` restores them; nothing is written
to the config.

`make sysprep` prints the two root-level changes worth making and
applies them with `doas scripts/sysprep.sh --apply`: a 4 GB disk
swapfile at low priority behind zram, so a full zram overflows to disk
instead of the OOM killer; and zram compression lz4 to zstd for a
better ratio (takes effect at the next boot; the file it edits is
copal's and a redeploy may restore it). The lever neither script can
pull is the VM's memory in UTM: 8 GB instead of 6 would end this class
of problem.

## The window in this VM

Measured 12 October 2026, evening, after `cargo test --workspace`
had built the dev-profile engine: `target/debug/treadcore-warm` panics
in `bevy_render` at startup, "unable to find a GPU". wgpu's Vulkan
backend finds the loader (`vulkan-loader`) and no driver behind it:
there is no `/usr/share/vulkan/icd.d/`, because no Mesa Vulkan driver
is installed. `WGPU_BACKEND=gl` fails the same way. The guest has
Mesa's GL (`mesa-gl`, `mesa-egl`, `mesa-dri-gallium`) but not its
software Vulkan rasteriser.

The fix is one package, and it needs root:

    doas apk add mesa-vulkan-swrast     # lavapipe: Vulkan on the CPU

Then `target/debug/treadcore-warm` again. If it opens, `make run` will.
Slow (every pixel is the CPU's), but the overlay is lines and 16 384
small sprites, which llvmpipe draws at a playable rate on four cores.
The alternative the roadmap names, developing the window on the host machine
host, stays open: the headless crates do not care either way.

**Done, 13 October 2026** (`LAB-REPORT-PLAYTEST.md`): the package went
in, and copal's full monty now installs it. Then every Bevy binary
segfaulted before its first frame, in `llvm::AsmPrinter::doFinalization`
on a thread lavapipe made to compile a shader: musl gives a thread
128 KiB of stack and LLVM's code generator wants more. The fix is
`scripts/bigstack.c`, a constructor that raises the default thread
stack to 8 MiB with `pthread_setattr_default_np`; `make configure`
builds it into `build/bigstack.so` on a musl machine (and reports the
Vulkan drivers it finds), and `make run` and `make lan` preload it.
With it the window holds 20 ticks a second on four cores at 86 to 93 %
of the CPU. A glibc machine needs none of this, and gets none.

Also measured: with 8 GB (the VM was given 8192 MB in UTM after the
first day), `make configure` sizes 4 jobs, and `cargo test
--workspace` compiled the remaining 170 engine crates plus every game
crate in 4 min 28 s wall (13 min CPU) with swap untouched. The memory
lesson of day one was about 6 GB; at 8 GB it does not bite.

## What is deliberately not done


- **sccache / a shared cache.** Useful across machines; nothing to gain
  on one.
- **Bevy's `dynamic_linking` feature.** Cuts link time for iteration,
  but this target is musl, where dynamic linking of Rust crates is the
  less-travelled road. Worth trying on a glibc desktop.
- **A faster linker (mold, lld).** Neither is installed here. On a
  machine that has one, `-C link-arg=-fuse-ld=mold` in
  `.cargo/config.toml` is the single best link-time change available.
- **Pre-built binaries.** The point of `make warm` is that the engine is
  built once per machine and never again until the pin moves.
