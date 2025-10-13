use go_updater::map_arch;

#[test]
fn map_arch_covers_all_arms() {
    assert_eq!(map_arch("aarch64"), "arm64"); // baris khusus
    assert_eq!(map_arch("x86_64"), "amd64"); // sanity
    assert_eq!(map_arch("riscv64"), "riscv64"); // cabang other => other
}
