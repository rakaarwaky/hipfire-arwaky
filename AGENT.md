# AGENT.md — hipfire-arwaky Project Guide

## Project Goal

Create a **minimal, focused fork of hipfire** that runs **Qwen3.5 on AMD RDNA2 (RX 6800 XT / gfx1030/1031) on Fedora Linux**.

### Why This Fork Exists

| Upstream (hipfire)                                                | This Fork (hipfire-arwaky)                             |
| ----------------------------------------------------------------- | ------------------------------------------------------ |
| Supports 10+ architectures (gfx906–gfx1201)                      | **Only RDNA2 (gfx1030/1031)**                    |
| 15+ architecture crates (llama, qwen2, deepseek4, dots-ocr, etc.) | **Only Qwen3.5 (hipfire-arch-qwen35)**           |
| 500k+ lines of kernel code for all arches                         | **Only kernels needed for gfx1030**              |
| Complex CI, many platforms                                        | **Single target: Fedora + RX 6800 XT**           |
| Hard to debug / iterate                                           | **Patch-on-compile: zero upstream modification** |

**Core philosophy**: upstream is read-only. We symlink only essential crates, patch at build time, and keep the ability to `git submodule update --remote` for upstream improvements.

---

## Architecture Overview

```
hipfire-arwaky/
├── hipfire-master/                 # Git submodule (READ-ONLY upstream)
│   └── crates/                     # All upstream crates
├── crates/                         # SYMLINKS to upstream (only what we need)
│   ├── hip-bridge*                 # FFI to AMD HIP/HSA via dlopen
│   ├── rdna-compute*               # Kernel compilation, caching, dispatch for RDNA
│   ├── hipfire-dispatch*           # Runtime ↔ rdna-compute dispatch layer
│   ├── hipfire-quantize*           # Quant utilities (MQ3/MQ4/HFQ4)
│   ├── hipfire-detect*             # Auto-detect GPU capabilities
│   ├── hipfire-atlas*              # Kernel autotuning
│   ├── hipfire-tui*                # Optional TUI config editor
│   └── hipfire-arch-qwen35*        # Qwen3.5 architecture (dense + MoE)
├── local-patched/                  # GENERATED (gitignored) - patched copies
│   ├── hipfire-runtime*            # Orchestrator (dev-deps stripped)
│   └── hipfire-arch-qwen35*        # Qwen3.5 arch (copied as-is)
├── patches/                        # Version-controlled patch files
│   ├── hipfire-runtime/
│   │   └── cargo-toml.patch        # Strip unused arch dev-deps
│   └── (future patches per crate)
├── xtask/                          # Build tooling (Rust crate)
│   └── src/main.rs                 # `cargo xtask patch` — copy + apply patches
├── Cargo.toml                      # Workspace with [patch] redirect
└── .gitignore
```

---

## Essential Crates (Qwen3.5 + RDNA2)

| Crate                   | Role                                                                    | Why Required                                                   |
| ----------------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------- |
| `hip-bridge`          | Safe FFI to `libamdhip64.so` / `libhsa-runtime64.so` via `dlopen` | Foundation — all GPU access                                   |
| `rdna-compute`        | Kernel JIT compile, cache, dispatch for RDNA                            | Contains `is_gfx1030`/`is_gfx1031` atoms, MMQ/DP4A kernels |
| `hipfire-dispatch`    | Runtime ↔ rdna-compute glue                                            | Feature `deltanet`, `from-hip-error`                       |
| `hipfire-runtime`     | Inference orchestrator                                                  | Feature `arch-qwen35` + `deltanet`                         |
| `hipfire-arch-qwen35` | Qwen3.5 architecture impl                                               | Forward pass, weight loading, KV, speculative decode           |
| `hipfire-quantize`    | Quant format helpers                                                    | MQ3/MQ4/HFQ4 encoding/decoding                                 |
| `hipfire-detect`      | GPU capability detection                                                | Auto-detect gfx1030 at runtime                                 |
| `hipfire-atlas`       | Kernel autotuning                                                       | Perf optimization for RDNA2                                    |
| `hipfire-tui`         | Terminal UI for config                                                  | Interactive config editor (optional)                           |

---

## Patch-on-Compile System

### How It Works

```bash
# 1. Update upstream
git submodule update --remote hipfire-master

# 2. Apply patches to local copies
cargo xtask patch  # copies symlinked crates → local-patched/ + applies patches/

# 3. Build (Cargo uses [patch] redirect to local-patched/)
cargo build --release -p hipfire-runtime --example infer_qwen35 --features arch-qwen35,deltanet
```

### Patch Files Location

```
patches/
├── hipfire-runtime/
│   └── cargo-toml.patch    # Removes unused arch dev-deps, keeps only qwen35
├── hipfire-arch-qwen35/    # Future: RDNA2-specific kernel fixes
├── rdna-compute/           # Future: gfx1030 tuning
└── ...
```

### Cargo `[patch]` Redirect

```toml
# Cargo.toml workspace root
[patch."https://github.com/Kaden-Schutt/hipfire"]
hip-bridge = { path = "local-patched/hip-bridge" }
rdna-compute = { path = "local-patched/rdna-compute" }
hipfire-dispatch = { path = "local-patched/hipfire-dispatch" }
hipfire-runtime = { path = "local-patched/hipfire-runtime" }
hipfire-arch-qwen35 = { path = "local-patched/hipfire-arch-qwen35" }
hipfire-quantize = { path = "local-patched/hipfire-quantize" }
```

**Key property**: upstream source in `hipfire-master/` is **never modified**. All changes live in `patches/` (version-controlled) and `local-patched/` (generated, gitignored).

---

## Workflow Commands

```bash
# Full rebuild after upstream update
git submodule update --remote hipfire-master
cargo xtask patch --force
cargo build --release

# Dry-run to see what would be patched
cargo xtask patch --dry-run

# Patch only specific crate
cargo xtask patch --crates hipfire-runtime

# Clean generated patched crates
cargo xtask clean --yes

# List status of all essential crates
cargo xtask list

# Run Qwen3.5 inference
cargo run --release -p hipfire-runtime --example infer_qwen35 --features arch-qwen35,deltanet -- /path/to/model.hfq

# Run daemon
cargo run --release -p hipfire-runtime --example daemon --features arch-qwen35,deltanet

# Launch TUI config editor
cargo run --release -p hipfire-tui
```

---

## Configuration

Single config file shared by all binaries:

```toml
# ~/.config/hipfire/config.toml
[device]
gfx_target = "gfx1030"      # RX 6800 XT
vram_limit_gb = 16

[model.qwen35]
path = "/models/qwen3.5-7b-mq4.hfq"
quant = "mq4"
context_len = 8192

[dflash]
enabled = true
draft_model = "/models/qwen3.5-0.5b.hfq"

[runtime]
log_level = "info"
```

All entry points (`infer_qwen35`, `daemon`, `hipfire-tui`) read this same file.

---

## RDNA2 (gfx1030) Kernel Notes

Upstream already provides gfx1030 kernels in `hipfire-master/kernels/src/`:

| Kernel Type      | Files                                        |
| ---------------- | -------------------------------------------- |
| GEMV HFQ4        | `gemv_hfq4g256.gfx1030.v1-v5.hip`          |
| GEMM MoE Gate/Up | `gemm_gate_up_hfq4g256_mmq_x*.gfx1030.hip` |
| GEMM QKV         | `gemm_qkv_hfq4g256_mmq_x*.gfx1030.hip`     |
| GEMM HFQ3        | `gemm_*_hfq3g256_*.gfx1030.hip`            |

These are dispatched automatically via `rdna-compute` when `ArchCaps::is_gfx1030` is true.

---

## Adding a New Patch

1. Create patch file in `patches/<crate-name>/`:
   ```bash
   cd /home/raka/App/hipfire-arwaky
   # Edit local-patched/<crate>/ manually, then generate patch:
   diff -u crates/<crate>/Cargo.toml local-patched/<crate>/Cargo.toml > patches/<crate>/cargo-toml.patch
   ```
2. Test: `cargo xtask patch --force --crates <crate>`
3. Verify: `cargo check -p <crate>`

---

## Upstream Update Procedure

```bash
# 1. Pull latest upstream
cd hipfire-master && git pull origin master && cd ..

# 2. Re-apply patches
cargo xtask patch --force

# 3. Build & test
cargo build --release -p hipfire-runtime --example infer_qwen35 --features arch-qwen35,deltanet

# 4. If build fails: adjust patches in patches/ to match upstream changes
# 5. Commit updated patches
git add patches/
git commit -m "chore: update patches for upstream <commit-hash>"
```

---

## Directory Purpose Summary

| Path                | Purpose                           | Git Tracked?              |
| ------------------- | --------------------------------- | ------------------------- |
| `hipfire-master/` | Upstream source (submodule)       | Yes (as submodule ref)    |
| `crates/*.rs`     | Symlinks to upstream crates       | Yes (symlinks)            |
| `local-patched/`  | Patched copies for build          | **No** (gitignored) |
| `patches/`        | Version-controlled patch files    | **Yes**             |
| `xtask/`          | Build tooling                     | **Yes**             |
| `Cargo.toml`      | Workspace config with `[patch]` | **Yes**             |

---

## Troubleshooting

| Issue                                                             | Fix                                                                                   |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `cannot specify features for packages outside workspace`        | Add crate to `members` in `Cargo.toml`                                            |
| `failed to load manifest for dependency hipfire-arch-deepseek4` | Patch `hipfire-runtime` dev-deps (see `patches/hipfire-runtime/cargo-toml.patch`) |
| `git apply failed`                                              | Check patch line numbers match current upstream; regenerate with `diff -u`          |
| Kernels not found for gfx1030                                     | Verify `rdna-compute` feature `deltanet` enabled; check `ArchCaps::is_gfx1030`  |

---

## Reference Links

- Upstream: https://github.com/Kaden-Schutt/hipfire
- RDNA2 ISA: https://gpuopen.com/learn/rdna2-instruction-set-architecture/
- Qwen3.5 Architecture: https://huggingface.co/Qwen/Qwen3.5-7B
