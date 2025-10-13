use go_updater::GoSemver;
use std::cmp::Ordering;

#[test]
fn semver_eq_and_cmp_paths() {
    let a = GoSemver::parse("go1.2.3").unwrap();
    let b = GoSemver::parse("go1.2.3").unwrap();
    let c = GoSemver::parse("go1.2.4").unwrap();

    // panggil eq secara eksplisit agar baris fn eq(...) terhitung
    assert!(<GoSemver as PartialEq<GoSemver>>::eq(&a, &b));
    assert!(!<GoSemver as PartialEq<GoSemver>>::eq(&a, &c));

    // panggil cmp secara eksplisit agar baris fn cmp(...) terhitung
    assert_eq!(<GoSemver as Ord>::cmp(&a, &b), Ordering::Equal);
    assert_eq!(<GoSemver as Ord>::cmp(&a, &c), Ordering::Less);
    assert_eq!(a.cmp(&c), Ordering::Less); // jalur method sugar
}
