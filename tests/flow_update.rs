use anyhow::Result;
use go_updater::*;
use std::cell::Cell;
use std::path::PathBuf;

struct MockHttpOk {
    json: String,
    downloaded: Cell<bool>,
}
impl Http for MockHttpOk {
    fn get_json(&self, _: &str) -> Result<String> {
        Ok(self.json.clone())
    }
    fn download(&self, _: &str, _: &PathBuf) -> Result<()> {
        self.downloaded.set(true);
        Ok(())
    }
}
struct MockSysInstall {
    called_root: Cell<bool>,
    post_version: String,
}
impl Sys for MockSysInstall {
    fn go_version(&self, path: Option<&str>) -> Result<String> {
        if path.is_some() {
            Ok(self.post_version.clone())
        } else {
            Ok("go1.24.8".into())
        }
    }
    fn is_root(&self) -> bool {
        true
    }
    fn run_root(&self, _: &str) -> Result<()> {
        self.called_root.set(true);
        Ok(())
    }
}
struct MockFsOk;
impl Fs for MockFsOk {
    fn verify_sha256(&self, _: &PathBuf, _: &str) -> Result<()> {
        Ok(())
    }
    fn tmp_path(&self, filename: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/{filename}"))
    }
}

#[test]
fn outdated_should_download_install_and_verify() -> Result<()> {
    let json = r#"[{"version":"go1.25.2","stable":true,"files":[
        {"filename":"go1.25.2.linux-amd64.tar.gz","os":"linux","arch":"amd64","sha256":"deadbeef","kind":"archive","size":null}
    ]}]"#;
    let http = MockHttpOk {
        json: json.into(),
        downloaded: Cell::new(false),
    };
    let sys = MockSysInstall {
        called_root: Cell::new(false),
        post_version: "go1.25.2".into(),
    };
    let fs = MockFsOk;

    run_update(&http, &sys, &fs)?;
    assert!(http.downloaded.get());
    assert!(sys.called_root.get());
    Ok(())
}
