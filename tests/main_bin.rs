use assert_cmd::prelude::*;
// cargo_bin() + assert()
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
// <-- penting: Command dari std
use tempfile::tempdir;

#[test]
fn main_binary_runs_happy_path() -> Result<(), Box<dyn std::error::Error>> {
    // 1) JSON inline → 1 rilis stable cocok linux-amd64 (tanpa network)
    unsafe {
        std::env::set_var(
            "GO_UPDATER_JSON_INLINE",
            r#"
    [
      {"version":"go1.2.3","stable":true,"files":[
        {"filename":"go1.2.3.linux-amd64.tar.gz","os":"linux","arch":"amd64",
         "sha256":"deadbeef","kind":"archive","size":null}
      ]}
    ]"#,
        );
    }

    // 2) 'go' palsu di PATH → "go version go1.2.3 linux/amd64"
    let td = tempdir()?;
    let go_path = td.path().join("go");
    {
        let mut f = fs::File::create(&go_path)?;
        writeln!(f, "#!/bin/sh")?;
        writeln!(f, "echo 'go version go1.2.3 linux/amd64'")?;
    }
    let mut perm = fs::metadata(&go_path)?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(&go_path, perm)?;
    let old_path = std::env::var("PATH").unwrap_or_default();

    unsafe {
        std::env::set_var("PATH", format!("{}:{}", td.path().display(), old_path));

        // 3) biar aman (meski jalur update tak dipakai karena up-to-date)
        std::env::set_var("GO_UPDATER_ASSUME_ROOT", "1");
    }

    // 4) jalankan binari crate ini
    Command::cargo_bin("go-updater")? // nama harus cocok dengan [[bin]] di Cargo.toml
        .assert()
        .success();

    unsafe {
        // bersihkan ENV
        std::env::remove_var("GO_UPDATER_JSON_INLINE");
        std::env::remove_var("GO_UPDATER_ASSUME_ROOT");
    }

    Ok(())
}
