use anyhow::Result;
use go_updater::*;
use std::path::PathBuf;

struct MockHttp {
    json: String,
}
impl Http for MockHttp {
    fn get_json(&self, _: &str) -> Result<String> {
        Ok(self.json.clone())
    }
    fn download(&self, _: &str, _: &PathBuf) -> Result<()> {
        Ok(())
    }
}
struct MockSysUpToDate;
impl Sys for MockSysUpToDate {
    fn go_version(&self, _: Option<&str>) -> Result<String> {
        Ok("go1.25.2".into())
    }
    fn is_root(&self) -> bool {
        true
    }
    fn run_root(&self, _: &str) -> Result<()> {
        Ok(())
    }
}
struct MockFs;
impl Fs for MockFs {
    fn verify_sha256(&self, _: &PathBuf, _: &str) -> Result<()> {
        Ok(())
    }
    fn tmp_path(&self, filename: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/{filename}"))
    }
}

#[test]
fn already_latest_should_do_nothing() -> Result<()> {
    let json = r#"[{"version":"go1.25.2","stable":true,"files":[
        {"filename":"go1.25.2.linux-amd64.tar.gz","os":"linux","arch":"amd64","sha256":"deadbeef","kind":"archive","size":null}
    ]}]"#;
    let http = MockHttp { json: json.into() };
    let sys = MockSysUpToDate;
    let fs = MockFs;
    // tidak error → sukses
    run_update(&http, &sys, &fs)
}
