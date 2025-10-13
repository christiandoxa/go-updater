# 🦀 Go Updater (Rust CLI)

[![Build](https://github.com/christiandoxa/go-updater/actions/workflows/ci.yml/badge.svg)](https://github.com/christiandoxa/go-updater/actions/workflows/ci.yml)
[![Coverage](https://github.com/christiandoxa/go-updater/actions/workflows/coverage.yml/badge.svg)](https://github.com/christiandoxa/go-updater/actions/workflows/coverage.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

CLI sederhana berbasis **Rust** untuk mengecek dan memperbarui instalasi **Go (Golang)** di Linux secara otomatis.
Program ini akan:

1. Mengambil daftar rilis terbaru dari `https://go.dev/dl/?mode=json`
2. Membandingkan dengan versi Go lokal (`go version`)
3. Jika versi lokal lebih lama, unduh versi terbaru ke `/tmp`
4. Verifikasi checksum **SHA256**
5. Instal ke `/usr/local/go` (dengan `sudo`/`pkexec`/`su`)
6. Verifikasi ulang instalasi (`go version`)

> **Edition Rust:** 2024

---

## 🚀 Fitur

* Deteksi otomatis versi Go yang terpasang
* Download + verifikasi SHA256
* Eskalasi privilese otomatis (`sudo` → `pkexec` → `su`)
* Verifikasi **pasca-instal**
* Arsitektur dipetakan otomatis (x86_64→amd64, aarch64→arm64)

---

## 🧩 Persyaratan

* Linux / Unix-like
* Rust & Cargo
* Internet aktif
* Hak `sudo` untuk instal ke `/usr/local`

---

## 🛠️ Build & Jalankan

```bash
git clone https://github.com/christiandoxa/go-updater.git
cd go-updater
cargo build --release
./target/release/go-updater
```

Contoh output:

```
Rilis stable terbaru di go.dev: go1.25.2
Versi Go lokal: go1.24.8
Akan update ke go1.25.2 dengan: go1.25.2.linux-amd64.tar.gz
Terunduh: /tmp/go1.25.2.linux-amd64.tar.gz
SHA256 OK (...)
Menjalankan instalasi: rm -rf /usr/local/go && tar -C /usr/local -xzf /tmp/go1.25.2.linux-amd64.tar.gz
Verifikasi OK: go1.25.2
Selesai. Pastikan PATH memuat /usr/local/go/bin
```

---

## 🧰 Struktur Proyek

```
go-updater/
├── Cargo.toml
├── .gitignore
├── LICENSE
├── README.md
├── src/
│   ├── lib.rs         # inti logic
│   └── main.rs        # entrypoint tipis (memanggil cli_main)
└── tests/
    ├── main_bin.rs                # integration test untuk binari
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

Jika `go` belum dikenali setelah instal:

```bash
echo 'export PATH=$PATH:/usr/local/go/bin' >> ~/.bashrc
source ~/.bashrc
```

---

## 🧪 Testing

Semua test ada di folder `tests/` (integration + unit via API publik).

```bash
cargo test
```

Catatan:

* Test **tidak** membutuhkan jaringan/akses root beneran: ia memakai env override:

    * `GO_UPDATER_JSON_INLINE` → menyuntik JSON rilis inline
    * `GO_UPDATER_ASSUME_ROOT=1` → menganggap proses sebagai root pada jalur tertentu
    * `GO_UPDATER_SUDO`, `GO_UPDATER_PKEXEC`, `GO_UPDATER_SU` → menunjuk biner palsu saat menguji fallback eskalasi

---

## ✅ Coverage 100%

Menggunakan **cargo-tarpaulin**.

### Lokal

1. Install tarpaulin:

```bash
cargo install cargo-tarpaulin
```

2. Jalankan coverage (gagal bila < 100%):

```bash
cargo tarpaulin --engine llvm --run-types Tests,Bins --fail-under 100
```

Keterangan:

* `--run-types Tests,Bins` memastikan **baris di `main.rs` (bin target)** juga terinstrumentasi, bukan hanya test
  binaries.
* Test integration `tests/main_bin.rs` menjalankan binari via `assert_cmd::cargo_bin`.

---

## 🔒 Env Override (untuk test/CI)

* `GO_UPDATER_JSON_INLINE` — JSON rilis (string) untuk menggantikan fetch HTTP
* `GO_UPDATER_JSON_URL` — URL alternatif (default: `https://go.dev/dl/?mode=json`)
* `GO_UPDATER_ASSUME_ROOT=1` — memaksa `is_root()` true
* `GO_UPDATER_SUDO`, `GO_UPDATER_PKEXEC`, `GO_UPDATER_SU` — path biner alternatif untuk eskalasi

---

## 🪪 Lisensi

MIT © 2025 — Christian Doxa Hamasiah
