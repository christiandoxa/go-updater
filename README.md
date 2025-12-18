# 🦀 Go Updater (Rust CLI)

[![Build](https://github.com/christiandoxa/go-updater/actions/workflows/ci.yml/badge.svg)](https://github.com/christiandoxa/go-updater/actions/workflows/ci.yml)
[![Coverage](https://github.com/christiandoxa/go-updater/actions/workflows/coverage.yml/badge.svg)](https://github.com/christiandoxa/go-updater/actions/workflows/coverage.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A simple **Rust** CLI to check and update **Go (Golang)** on Linux automatically.
The program will:

1. Fetch the latest stable releases from `https://go.dev/dl/?mode=json`
2. Compare with the local Go version (`go version`)
3. If the local version is older, download the latest release to `/tmp`
4. Verify the **SHA256** checksum
5. Install to `/usr/local/go` (via `sudo`/`pkexec`/`su`)
6. Re-verify the installation (`go version`)

> **Rust edition:** 2024

---

## 🚀 Features

* Auto-detect installed Go version
* Download + SHA256 verification
* Automatic privilege escalation (`sudo` → `pkexec` → `su`)
* Post-install verification
* Auto architecture mapping (x86_64→amd64, aarch64→arm64)

---

## 🧩 Requirements

* Linux / Unix-like
* Rust & Cargo
* Internet connection
* `sudo` access to install to `/usr/local`

---

## 🛠️ Build & Run

```bash
git clone https://github.com/christiandoxa/go-updater.git
cd go-updater
cargo build --release
./target/release/go-updater
```

Sample output:

```
Latest stable release on go.dev: go1.25.2
Local Go version: go1.24.8
Will update to go1.25.2 with: go1.25.2.linux-amd64.tar.gz
Downloaded: /tmp/go1.25.2.linux-amd64.tar.gz
SHA256 OK (...)
Running install: rm -rf /usr/local/go && tar -C /usr/local -xzf /tmp/go1.25.2.linux-amd64.tar.gz
Verification OK: go1.25.2
Done. Make sure PATH includes /usr/local/go/bin
```

---

## 🧰 Project Structure

```
go-updater/
├── Cargo.toml
├── .gitignore
├── LICENSE
├── README.md
├── src/
│   ├── lib.rs         # core logic
│   └── main.rs        # thin entrypoint (calls cli_main)
└── tests/
    ├── main_bin.rs                # integration test for the binary
    ├── real_impls.rs              # RealHttp/RealSys/RealFs
    ├── real_sys_run_root.rs       # sudo/pkexec/su
    ├── real_sys_run_root_missing.rs
    ├── semver_eq_cmp.rs
    ├── fallback_and_mismatch.rs
    ├── map_arch.rs
    └── *.rs
```

---

## ⚙️ PATH

If `go` is not found after installation:

```bash
echo 'export PATH=$PATH:/usr/local/go/bin' >> ~/.bashrc
source ~/.bashrc
```

---

## 🧪 Testing

All tests live under `tests/` (integration + unit via the public API).

```bash
cargo test
```

Notes:

* Tests **do not** require real network/root access: use the env overrides below when testing:

  * `GO_UPDATER_JSON_INLINE` → inject release JSON inline
  * `GO_UPDATER_ASSUME_ROOT=1` → treat process as root on certain paths
  * `GO_UPDATER_SUDO`, `GO_UPDATER_PKEXEC`, `GO_UPDATER_SU` → point to fake binaries to test escalation fallback

---

## ✅ Coverage (with `cargo-llvm-cov`)

### Run locally

1. Install:

```bash
cargo install cargo-llvm-cov
```

2. Generate report & open HTML:

```bash
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-features --html --open
```

3. (Optional) Gate coverage with a threshold:

```bash
# fail if line coverage < 100%
cargo llvm-cov --workspace --all-features --fail-under-lines 100
```

> Tip: if you also want an LCOV file for external tooling:
>
> ```bash
> cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
> ```

---

## 🔒 Env Overrides (for tests/CI)

* `GO_UPDATER_JSON_INLINE` — release JSON string to replace the HTTP fetch
* `GO_UPDATER_JSON_URL` — alternative URL (default: `https://go.dev/dl/?mode=json`)
* `GO_UPDATER_ASSUME_ROOT=1` — force `is_root()` true
* `GO_UPDATER_SUDO`, `GO_UPDATER_PKEXEC`, `GO_UPDATER_SU` — alternative paths for escalation binaries

---

## 🪪 License

MIT © 2025 — Christian Doxa Hamasiah
