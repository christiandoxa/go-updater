use anyhow::{Result, anyhow};
use go_updater::*;
use std::path::PathBuf;

struct MockHttpJsonBad;
impl Http for MockHttpJsonBad {
    fn get_json(&self, _: &str) -> Result<String> {
        Ok("not json".into())
    }
    fn download(&self, _: &str, _: &PathBuf) -> Result<()> {
        Ok(())
    }
}
struct MockSysNoGo;
impl Sys for MockSysNoGo {
    fn go_version(&self, _: Option<&str>) -> Result<String> {
        Err(anyhow!("no go"))
    }
    fn is_root(&self) -> bool {
        true
    }
    fn run_root(&self, _: &str) -> Result<()> {
        Ok(())
    }
}
struct MockFsAny;
impl Fs for MockFsAny {
    fn verify_sha256(&self, _: &PathBuf, _: &str) -> Result<()> {
        Ok(())
    }
    fn tmp_path(&self, filename: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/{filename}"))
    }
}

#[test]
fn bad_json_should_error() {
    let http = MockHttpJsonBad;
    let sys = MockSysNoGo;
    let fs = MockFsAny;
    let err = run_update(&http, &sys, &fs).unwrap_err();
    assert!(err.to_string().contains("expected"));
}
