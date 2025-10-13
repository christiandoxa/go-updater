use anyhow::Result;
use go_updater::{RealSys, Sys};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

fn mk_exe(exit_code: i32) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("cmd.sh");
    let mut f = File::create(&path).unwrap();
    writeln!(f, "#!/bin/sh").unwrap();
    writeln!(f, "exit {}", exit_code).unwrap();
    drop(f);
    let mut perm = fs::metadata(&path).unwrap().permissions();
    perm.set_mode(0o755);
    fs::set_permissions(&path, perm).unwrap();
    (dir, path)
}

#[test]
fn run_root_missing_sudo_then_pkexec_ok() -> Result<()> {
    unsafe {
        std::env::remove_var("GO_UPDATER_ASSUME_ROOT");

        // set sudo ke path yang tidak ada → memicu try_run(...).unwrap_or(false)
        std::env::set_var("GO_UPDATER_SUDO", "/definitely/not/exist/sudo");
    }

    // pkexec ada dan sukses
    let (_d1, pkexec_ok) = mk_exe(0);
    unsafe {
        std::env::set_var("GO_UPDATER_PKEXEC", pkexec_ok.to_string_lossy().to_string());
    }

    // su gagal (tidak akan dipanggil karena pkexec sukses lebih dulu)
    let (_d2, su_no) = mk_exe(1);
    unsafe {
        std::env::set_var("GO_UPDATER_SU", su_no.to_string_lossy().to_string());
    }

    let sys = RealSys;
    sys.run_root("echo ok")?; // sukses via pkexec

    unsafe {
        // bersihkan
        std::env::remove_var("GO_UPDATER_SUDO");
        std::env::remove_var("GO_UPDATER_PKEXEC");
        std::env::remove_var("GO_UPDATER_SU");
    }

    Ok(())
}
