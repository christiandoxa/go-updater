use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, copy};
use std::path::PathBuf;
use std::process::Command;

/// ==== Data from go.dev ====
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
            .ok_or_else(|| anyhow!("not a Go format: {tag}"))?;
        let parts: Vec<_> = s.split('.').collect();
        let major = parts
            .get(0)
            .ok_or_else(|| anyhow!("invalid version"))?
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

/// ==== I/O abstractions for mocking ====
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

/// Real implementations
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
        let caps = re
            .captures(&s)
            .ok_or_else(|| anyhow!("parse failed: {s}"))?;
        Ok(caps.get(1).unwrap().as_str().to_string())
    }

    fn is_root(&self) -> bool {
        if std::env::var("GO_UPDATER_ASSUME_ROOT").as_deref() == Ok("1") {
            return true;
        }
        unsafe { libc::geteuid() == 0 }
    }

    fn run_root(&self, cmd: &str) -> Result<()> {
        // Override program names via env (for tests).
        let sudo_prog = std::env::var("GO_UPDATER_SUDO").unwrap_or_else(|_| "sudo".into());
        let pkexec_prog = std::env::var("GO_UPDATER_PKEXEC").unwrap_or_else(|_| "pkexec".into());
        let su_prog = std::env::var("GO_UPDATER_SU").unwrap_or_else(|_| "su".into());

        // Run directly as root.
        if self.is_root() {
            let st = Command::new("sh").arg("-c").arg(cmd).status()?;
            return if st.success() {
                Ok(())
            } else {
                Err(anyhow!("command failed (root)"))
            };
        }

        // Try sudo -> pkexec with helper (swallow ENOENT).
        if try_run(&sudo_prog, &["sh", "-c", cmd]).unwrap_or(false) {
            return Ok(());
        }
        if try_run(&pkexec_prog, &["sh", "-c", cmd]).unwrap_or(false) {
            return Ok(());
        }

        // Last: su (keep context in error for clarity).
        let st = Command::new(&su_prog)
            .arg("-c")
            .arg(cmd)
            .status()
            .context("privilege escalation failed (sudo/pkexec/su)")?;
        if st.success() {
            Ok(())
        } else {
            Err(anyhow!("command failed (su)"))
        }
    }
}

// Generic helper to run a program and return true on success.
// Returns Err(io::Error) if spawn fails; call-site may .unwrap_or(false).
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

/// ==== Main API under test ====
pub fn run_update<H: Http, S: Sys, F: Fs>(http: &H, sys: &S, fs: &F) -> Result<()> {
    const TOTAL_STEPS: usize = 9;

    // 1) fetch JSON
    eprintln!("→ [1/{TOTAL_STEPS}] Fetching Go release list...");
    let (json, source_json) = if let Ok(inline) = std::env::var("GO_UPDATER_JSON_INLINE") {
        (inline, "env GO_UPDATER_JSON_INLINE".to_string())
    } else {
        let url = std::env::var("GO_UPDATER_JSON_URL")
            .unwrap_or_else(|_| "https://go.dev/dl/?mode=json".to_string());
        let text = http.get_json(&url)?;
        (text, format!("GET {url}"))
    };
    eprintln!("   • Source: {source_json}");

    // 2) parse & pick latest stable
    eprintln!("→ [2/{TOTAL_STEPS}] Picking latest stable release...");
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
        .ok_or_else(|| anyhow!("no stable release found"))?;
    let latest_sem = GoSemver::parse(&latest.version)?;
    eprintln!(
        "   • Latest: {} ({} artifacts)",
        latest.version,
        latest.files.len()
    );

    // 3) local version
    eprintln!("→ [3/{TOTAL_STEPS}] Checking local Go version...");
    let local = sys
        .go_version(None)
        .unwrap_or_else(|_| "go0.0.0".to_string());
    let local_sem = GoSemver::parse(&local).unwrap_or(GoSemver {
        major: 0,
        minor: 0,
        patch: 0,
    });
    eprintln!("   • Local version: {local}");

    // 4) version comparison
    eprintln!("→ [4/{TOTAL_STEPS}] Comparing versions...");
    if local_sem >= latest_sem {
        eprintln!("✓ Already up to date ({local}) — no update needed.");
        return Ok(());
    }
    eprintln!("   • Update needed: {local} -> {}", latest.version);

    // 5) pick linux-<arch>.tar.gz artifact
    let arch = map_arch(std::env::consts::ARCH);
    eprintln!("→ [5/{TOTAL_STEPS}] Picking artifact for linux-{arch} (.tar.gz) ...");
    let pick = latest
        .files
        .iter()
        .find(|f| {
            f.os == "linux"
                && f.arch == arch
                && f.kind == "archive"
                && f.filename.ends_with(".tar.gz")
        })
        .ok_or_else(|| anyhow!("linux-{arch} artifact not found"))?;
    eprintln!("   • Artifact: {}", pick.filename);

    // 6) download
    let url = format!("https://go.dev/dl/{}", pick.filename);
    let dest = fs.tmp_path(&pick.filename);
    eprintln!(
        "→ [6/{TOTAL_STEPS}] Downloading {} -> {}",
        url,
        dest.display()
    );
    http.download(&url, &dest)?;

    // 7) verify sha256
    eprintln!("→ [7/{TOTAL_STEPS}] Verifying SHA256...");
    fs.verify_sha256(&dest, &pick.sha256)?;
    eprintln!("   • OK: checksum matches");

    // 8) installation (root)
    eprintln!("→ [8/{TOTAL_STEPS}] Installing to /usr/local (requires admin rights)...");
    let cmd = format!(
        "rm -rf /usr/local/go && tar -C /usr/local -xzf {}",
        dest.display()
    );
    sys.run_root(&cmd)?;

    // 9) post-install verification
    eprintln!("→ [9/{TOTAL_STEPS}] Verifying installation (go version)...");
    let newv = sys
        .go_version(Some("/usr/local/go/bin/go"))
        .or_else(|_| sys.go_version(None))?;
    if newv != latest.version {
        return Err(anyhow!("verification failed: {newv} != {}", latest.version));
    }
    eprintln!("✓ Done: installed {}", newv);
    Ok(())
}

pub fn map_arch(arch: &'static str) -> &'static str {
    match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other, // <= this branch is now testable
    }
}

pub fn cli_main() -> Result<()> {
    let http = RealHttp;
    let sys = RealSys;
    let fs = RealFs;
    run_update(&http, &sys, &fs)
}
