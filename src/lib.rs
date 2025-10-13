use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, copy};
use std::path::PathBuf;
use std::process::Command;

/// ==== Data dari go.dev ====
#[derive(Debug, Deserialize, Clone)]
pub struct GoRelease {
    pub version: String,
    pub stable: bool,
    pub files: Vec<GoFile>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct GoFile {
    pub filename: String,
    pub os: String,
    pub arch: String,
    pub sha256: String,
    pub kind: String,
    pub size: Option<u64>,
}

/// ==== Semantic version ====
#[derive(Debug, Clone, Copy, Eq)]
pub struct GoSemver {
    major: u64,
    minor: u64,
    patch: u64,
}
impl GoSemver {
    pub fn parse(tag: &str) -> Result<Self> {
        let s = tag
            .strip_prefix("go")
            .ok_or_else(|| anyhow!("bukan format Go: {tag}"))?;
        let parts: Vec<_> = s.split('.').collect();
        let major = parts
            .get(0)
            .ok_or_else(|| anyhow!("versi tidak valid"))?
            .parse()?;
        let minor = parts.get(1).unwrap_or(&"0").parse()?;
        let patch = parts.get(2).unwrap_or(&"0").parse()?;
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}
impl PartialEq<Self> for GoSemver {
    fn eq(&self, other: &Self) -> bool {
        (self.major, self.minor, self.patch) == (other.major, other.minor, other.patch)
    }
}
impl PartialOrd<Self> for GoSemver {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some((self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch)))
    }
}
impl Ord for GoSemver {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

/// ==== Abstraksi I/O supaya bisa di-mock ====
pub trait Http {
    fn get_json(&self, url: &str) -> Result<String>;
    fn download(&self, url: &str, dest: &PathBuf) -> Result<()>;
}
pub trait Sys {
    fn go_version(&self, path: Option<&str>) -> Result<String>;
    fn is_root(&self) -> bool;
    fn run_root(&self, cmd: &str) -> Result<()>;
}
pub trait Fs {
    fn verify_sha256(&self, path: &PathBuf, expected_hex: &str) -> Result<()>;
    fn tmp_path(&self, filename: &str) -> PathBuf;
}

/// Implementasi nyata
pub struct RealHttp;
impl Http for RealHttp {
    fn get_json(&self, url: &str) -> Result<String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("go-updater/0.1")
            .build()?;
        let text = client.get(url).send()?.error_for_status()?.text()?;
        Ok(text)
    }
    fn download(&self, url: &str, dest: &PathBuf) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let mut resp = client.get(url).send()?.error_for_status()?;
        let mut file = File::create(dest)?;
        copy(&mut resp, &mut file)?;
        Ok(())
    }
}
pub struct RealSys;
impl Sys for RealSys {
    fn go_version(&self, path: Option<&str>) -> Result<String> {
        let mut cmd = Command::new(path.unwrap_or("go"));
        let out = cmd.arg("version").output()?;
        if !out.status.success() {
            return Err(anyhow!("`go version` exit code {:?}", out.status.code()));
        }
        let s = String::from_utf8_lossy(&out.stdout);
        let re = Regex::new(r"go version (go[0-9.]+)")?;
        let caps = re.captures(&s).ok_or_else(|| anyhow!("parse gagal: {s}"))?;
        Ok(caps.get(1).unwrap().as_str().to_string())
    }

    fn is_root(&self) -> bool {
        if std::env::var("GO_UPDATER_ASSUME_ROOT").as_deref() == Ok("1") {
            return true;
        }
        unsafe { libc::geteuid() == 0 }
    }

    fn run_root(&self, cmd: &str) -> Result<()> {
        // override nama program via env (untuk test)
        let sudo_prog = std::env::var("GO_UPDATER_SUDO").unwrap_or_else(|_| "sudo".into());
        let pkexec_prog = std::env::var("GO_UPDATER_PKEXEC").unwrap_or_else(|_| "pkexec".into());
        let su_prog = std::env::var("GO_UPDATER_SU").unwrap_or_else(|_| "su".into());

        // root langsung
        if self.is_root() {
            let st = Command::new("sh").arg("-c").arg(cmd).status()?;
            return if st.success() {
                Ok(())
            } else {
                Err(anyhow!("cmd gagal (root)"))
            };
        }

        // coba sudo → pkexec dengan helper (swallow error ENOENT)
        if try_run(&sudo_prog, &["sh", "-c", cmd]).unwrap_or(false) {
            return Ok(());
        }
        if try_run(&pkexec_prog, &["sh", "-c", cmd]).unwrap_or(false) {
            return Ok(());
        }

        // terakhir: su (tetap beri context di error agar jelas)
        let st = Command::new(&su_prog)
            .arg("-c")
            .arg(cmd)
            .status()
            .context("gagal eskalasi (sudo/pkexec/su)")?;
        if st.success() {
            Ok(())
        } else {
            Err(anyhow!("cmd gagal (su)"))
        }
    }
}

// Helper generik untuk menjalankan program dan mengembalikan true jika sukses.
// Mengembalikan Err(io::Error) jika gagal spawn; call-site boleh .unwrap_or(false).
fn try_run(prog: &str, args: &[&str]) -> std::io::Result<bool> {
    Command::new(prog).args(args).status().map(|s| s.success())
}

pub struct RealFs;
impl Fs for RealFs {
    fn verify_sha256(&self, path: &PathBuf, expected_hex: &str) -> Result<()> {
        let mut f = File::open(path)?;
        let mut h = Sha256::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        let got = hex::encode(h.finalize());
        if got != expected_hex.to_lowercase() {
            return Err(anyhow!("hash mismatch: expected {expected_hex}, got {got}"));
        }
        Ok(())
    }
    fn tmp_path(&self, filename: &str) -> PathBuf {
        PathBuf::from("/tmp").join(filename)
    }
}

/// ==== API utama yang di-test ====
pub fn run_update<H: Http, S: Sys, F: Fs>(http: &H, sys: &S, fs: &F) -> Result<()> {
    const TOTAL_STEPS: usize = 9;

    // 1) fetch JSON
    eprintln!("→ [1/{TOTAL_STEPS}] Mengambil daftar rilis Go…");
    let (json, sumber_json) = if let Ok(inline) = std::env::var("GO_UPDATER_JSON_INLINE") {
        (inline, "env GO_UPDATER_JSON_INLINE".to_string())
    } else {
        let url = std::env::var("GO_UPDATER_JSON_URL")
            .unwrap_or_else(|_| "https://go.dev/dl/?mode=json".to_string());
        let text = http.get_json(&url)?;
        (text, format!("GET {url}"))
    };
    eprintln!("   • Sumber: {sumber_json}");

    // 2) parse & pilih latest stable
    eprintln!("→ [2/{TOTAL_STEPS}] Memilih rilis stabil terbaru…");
    let releases: Vec<GoRelease> = serde_json::from_str(&json)?;
    let latest = releases
        .iter()
        .filter(|r| r.stable)
        .max_by(|a, b| {
            let va = GoSemver::parse(&a.version).unwrap_or(GoSemver {
                major: 0,
                minor: 0,
                patch: 0,
            });
            let vb = GoSemver::parse(&b.version).unwrap_or(GoSemver {
                major: 0,
                minor: 0,
                patch: 0,
            });
            va.cmp(&vb)
        })
        .ok_or_else(|| anyhow!("tidak ada stable release"))?;
    let latest_sem = GoSemver::parse(&latest.version)?;
    eprintln!(
        "   • Terbaru: {} ({} artefak)",
        latest.version,
        latest.files.len()
    );

    // 3) versi lokal
    eprintln!("→ [3/{TOTAL_STEPS}] Mengecek versi Go lokal…");
    let local = sys
        .go_version(None)
        .unwrap_or_else(|_| "go0.0.0".to_string());
    let local_sem = GoSemver::parse(&local).unwrap_or(GoSemver {
        major: 0,
        minor: 0,
        patch: 0,
    });
    eprintln!("   • Versi lokal: {local}");

    // 4) perbandingan versi
    eprintln!("→ [4/{TOTAL_STEPS}] Membandingkan versi…");
    if local_sem >= latest_sem {
        eprintln!("✓ Sudah terbaru ({local}) — tidak perlu update.");
        return Ok(());
    }
    eprintln!("   • Perlu update: {local} → {}", latest.version);

    // 5) pilih artefak linux-<arch>.tar.gz
    let arch = map_arch(std::env::consts::ARCH);
    eprintln!("→ [5/{TOTAL_STEPS}] Memilih artefak untuk linux-{arch} (.tar.gz) …");
    let pick = latest
        .files
        .iter()
        .find(|f| {
            f.os == "linux"
                && f.arch == arch
                && f.kind == "archive"
                && f.filename.ends_with(".tar.gz")
        })
        .ok_or_else(|| anyhow!("artefak linux-{arch} tidak ditemukan"))?;
    eprintln!("   • Artefak: {}", pick.filename);

    // 6) unduh
    let url = format!("https://go.dev/dl/{}", pick.filename);
    let dest = fs.tmp_path(&pick.filename);
    eprintln!("→ [6/{TOTAL_STEPS}] Mengunduh {} → {}", url, dest.display());
    http.download(&url, &dest)?;

    // 7) verifikasi sha256
    eprintln!("→ [7/{TOTAL_STEPS}] Verifikasi SHA256…");
    fs.verify_sha256(&dest, &pick.sha256)?;
    eprintln!("   • OK: checksum cocok");

    // 8) instalasi (root)
    eprintln!("→ [8/{TOTAL_STEPS}] Menginstal ke /usr/local (membutuhkan hak admin)…");
    let cmd = format!(
        "rm -rf /usr/local/go && tar -C /usr/local -xzf {}",
        dest.display()
    );
    sys.run_root(&cmd)?;

    // 9) verifikasi pasca-instal
    eprintln!("→ [9/{TOTAL_STEPS}] Verifikasi pemasangan (go version)…");
    let newv = sys
        .go_version(Some("/usr/local/go/bin/go"))
        .or_else(|_| sys.go_version(None))?;
    if newv != latest.version {
        return Err(anyhow!("verifikasi gagal: {newv} != {}", latest.version));
    }
    eprintln!("✓ Selesai: terpasang {}", newv);
    Ok(())
}

pub fn map_arch(arch: &'static str) -> &'static str {
    match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other, // <= cabang ini sekarang bisa kita uji
    }
}

pub fn cli_main() -> Result<()> {
    let http = RealHttp;
    let sys = RealSys;
    let fs = RealFs;
    run_update(&http, &sys, &fs)
}
