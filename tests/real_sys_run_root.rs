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
fn run_root_sudo_success() -> Result<()> {
    unsafe {
        std::env::remove_var("GO_UPDATER_ASSUME_ROOT");
    }

    let (_d1, sudo_ok) = mk_exe(0);
    let (_d2, pkexec_no) = mk_exe(1);
    let (_d3, su_no) = mk_exe(1);

    unsafe {
        std::env::set_var("GO_UPDATER_SUDO", sudo_ok.to_string_lossy().to_string());
        std::env::set_var("GO_UPDATER_PKEXEC", pkexec_no.to_string_lossy().to_string());
        std::env::set_var("GO_UPDATER_SU", su_no.to_string_lossy().to_string());
    }

    let sys = RealSys;
    sys.run_root("echo ok")?; // sukses via sudo

    // bersih

    unsafe {
        std::env::remove_var("GO_UPDATER_SUDO");
        std::env::remove_var("GO_UPDATER_PKEXEC");
        std::env::remove_var("GO_UPDATER_SU");
    }

    Ok(())
}

#[test]
fn run_root_pkexec_success_after_sudo_fail() -> Result<()> {
    unsafe {
        std::env::remove_var("GO_UPDATER_ASSUME_ROOT");
    }

    let (_d1, sudo_no) = mk_exe(1);
    let (_d2, pkexec_ok) = mk_exe(0);
    let (_d3, su_no) = mk_exe(1);

    unsafe {
        std::env::set_var("GO_UPDATER_SUDO", sudo_no.to_string_lossy().to_string());
        std::env::set_var("GO_UPDATER_PKEXEC", pkexec_ok.to_string_lossy().to_string());
        std::env::set_var("GO_UPDATER_SU", su_no.to_string_lossy().to_string());
    }

    let sys = RealSys;
    sys.run_root("echo ok")?; // sukses via pkexec

    unsafe {
        std::env::remove_var("GO_UPDATER_SUDO");
        std::env::remove_var("GO_UPDATER_PKEXEC");
        std::env::remove_var("GO_UPDATER_SU");
    }

    Ok(())
}

#[test]
fn run_root_su_success_after_sudo_and_pkexec_fail() -> Result<()> {
    unsafe {
        std::env::remove_var("GO_UPDATER_ASSUME_ROOT");
    }

    let (_d1, sudo_no) = mk_exe(1);
    let (_d2, pkexec_no) = mk_exe(1);
    let (_d3, su_ok) = mk_exe(0);

    unsafe {
        std::env::set_var("GO_UPDATER_SUDO", sudo_no.to_string_lossy().to_string());
        std::env::set_var("GO_UPDATER_PKEXEC", pkexec_no.to_string_lossy().to_string());
        std::env::set_var("GO_UPDATER_SU", su_ok.to_string_lossy().to_string());
    }

    let sys = RealSys;
    sys.run_root("echo ok")?; // sukses via su

    unsafe {
        std::env::remove_var("GO_UPDATER_SUDO");
        std::env::remove_var("GO_UPDATER_PKEXEC");
        std::env::remove_var("GO_UPDATER_SU");
    }
    Ok(())
}

#[test]
fn run_root_all_fail_hits_final_err() {
    unsafe {
        std::env::remove_var("GO_UPDATER_ASSUME_ROOT");
    }

    let (_d1, sudo_no) = mk_exe(1);
    let (_d2, pkexec_no) = mk_exe(1);
    let (_d3, su_no) = mk_exe(1);

    unsafe {
        std::env::set_var("GO_UPDATER_SUDO", sudo_no.to_string_lossy().to_string());
        std::env::set_var("GO_UPDATER_PKEXEC", pkexec_no.to_string_lossy().to_string());
        std::env::set_var("GO_UPDATER_SU", su_no.to_string_lossy().to_string());
    }

    let sys = RealSys;
    let err = sys.run_root("echo ok").unwrap_err();
    assert!(err.to_string().contains("cmd gagal (su)"));

    unsafe {
        std::env::remove_var("GO_UPDATER_SUDO");
        std::env::remove_var("GO_UPDATER_PKEXEC");
        std::env::remove_var("GO_UPDATER_SU");
    }
}
