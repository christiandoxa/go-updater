use go_updater::{RealSys, Sys};

#[test]
fn is_root_hits_geteuid() {
    // pastikan hook test tidak aktif
    unsafe {
        std::env::remove_var("GO_UPDATER_ASSUME_ROOT");
    }
    let sys = RealSys;
    // tidak peduli hasilnya true/false, yang penting baris unsafe dieksekusi
    let _ = sys.is_root();
}
