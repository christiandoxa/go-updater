use anyhow::Result;
use go_updater::{Fs, Http, RealFs, RealHttp, RealSys, Sys};
use httpmock::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::tempdir;

// ---------- RealHttp ----------
#[test]
fn real_http_get_json_ok() -> Result<()> {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/dl");
        then.status(200)
            .header("Content-Type", "application/json")
            .body(r#"{"hello":"world"}"#);
    });

    let http = RealHttp;
    let body = http.get_json(&format!("{}/dl", server.base_url()))?;
    assert!(body.contains("hello"));
    m.assert();
    Ok(())
}

#[test]
fn real_http_get_json_http_error() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/err");
        then.status(500).body("boom");
    });

    let http = RealHttp;
    let err = http
        .get_json(&format!("{}/err", server.base_url()))
        .unwrap_err();
    assert!(err.to_string().contains("500"));
    m.assert();
}

#[test]
fn real_http_download_ok() -> Result<()> {
    let server = MockServer::start();
    let payload = b"abc123";
    let m = server.mock(|when, then| {
        when.method(GET).path("/bin");
        then.status(200).body(payload.as_slice());
    });

    let http = RealHttp;
    let dir = tempdir()?;
    let dest = dir.path().join("out.bin");
    http.download(&format!("{}/bin", server.base_url()), &dest)?;
    let got = fs::read(&dest)?;
    assert_eq!(got, payload);
    m.assert();
    Ok(())
}

#[test]
fn real_http_download_http_error() {
    let server = MockServer::start();
    let m = server.mock(|when, then| {
        when.method(GET).path("/binerr");
        then.status(404);
    });
    let http = RealHttp;
    let dir = tempdir().unwrap();
    let dest = dir.path().join("x");
    let err = http
        .download(&format!("{}/binerr", server.base_url()), &dest)
        .unwrap_err();
    assert!(err.to_string().contains("404"));
    m.assert();
}

// ---------- RealFs ----------
#[test]
fn real_fs_verify_sha256_ok() -> Result<()> {
    use sha2::{Digest, Sha256};
    let fsimpl = RealFs;
    let dir = tempdir()?;
    let path = dir.path().join("file.txt");
    fs::write(&path, b"hello")?;
    // hitung hash expected
    let mut h = Sha256::new();
    h.update(b"hello");
    let expected = hex::encode(h.finalize());
    fsimpl.verify_sha256(&path, &expected)?;
    Ok(())
}

#[test]
fn real_fs_verify_sha256_mismatch() -> Result<()> {
    let fsimpl = RealFs;
    let dir = tempdir()?;
    let path = dir.path().join("file.txt");
    fs::write(&path, b"hello")?;
    let err = fsimpl.verify_sha256(&path, "deadbeef").unwrap_err();
    assert!(err.to_string().contains("hash mismatch"));
    Ok(())
}

#[test]
fn real_fs_tmp_path() {
    let fsimpl = RealFs;
    let p = fsimpl.tmp_path("abc.tar.gz");
    assert!(p.to_string_lossy().ends_with("/tmp/abc.tar.gz"));
}

// ---------- RealSys ----------
fn make_exe(contents: &str) -> Result<PathBuf> {
    let dir = tempdir()?;
    let path = dir.path().join("mockbin.sh");
    let mut f = File::create(&path)?;
    writeln!(f, "{}", contents)?;
    drop(f);
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms)?;
    // penting: jangan drop dir, jadi kita return path + _dir leak biar path tetap ada sepanjang test
    std::mem::forget(dir);
    Ok(path)
}

#[test]
fn real_sys_go_version_ok() -> Result<()> {
    let bin = make_exe("#!/bin/sh\necho 'go version go1.2.3 linux/amd64'\n")?;
    let sys = RealSys;
    let v = sys.go_version(Some(bin.to_str().unwrap()))?;
    assert_eq!(v, "go1.2.3");
    Ok(())
}

#[test]
fn real_sys_go_version_nonzero_exit() {
    let bin = make_exe("#!/bin/sh\nexit 1\n").unwrap();
    let sys = RealSys;
    let err = sys.go_version(Some(bin.to_str().unwrap())).unwrap_err();
    assert!(err.to_string().contains("exit code"));
}

#[test]
fn real_sys_go_version_bad_output() {
    let bin = make_exe("#!/bin/sh\necho 'not a go version'\n").unwrap();
    let sys = RealSys;
    let err = sys.go_version(Some(bin.to_str().unwrap())).unwrap_err();
    assert!(err.to_string().contains("parse gagal"));
}

#[test]
fn real_sys_run_root_success_and_fail_via_env() {
    // Paksa dianggap root → jalur "root langsung"
    unsafe {
        std::env::set_var("GO_UPDATER_ASSUME_ROOT", "1");
    }
    let sys = RealSys;

    // sukses
    sys.run_root("true").expect("harus sukses");

    // gagal
    let err = sys.run_root("false").unwrap_err();
    assert!(err.to_string().contains("cmd gagal (root)"));

    // bersihkan env
    unsafe {
        std::env::remove_var("GO_UPDATER_ASSUME_ROOT");
    }
}

#[test]
fn real_sys_is_root_smoke() {
    // sekadar memanggil untuk cover baris libc::geteuid()
    let sys = RealSys;
    let _ = sys.is_root();
}
