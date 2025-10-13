use anyhow::Result;
use go_updater::{Fs, Http, Sys, run_update};
use std::path::PathBuf;

struct H {
    json: String,
}
impl Http for H {
    fn get_json(&self, _: &str) -> Result<String> {
        Ok(self.json.clone())
    }
    fn download(&self, _: &str, _: &PathBuf) -> Result<()> {
        Ok(())
    }
}

struct S;
impl Sys for S {
    fn go_version(&self, path: Option<&str>) -> Result<String> {
        // sebelum instal → "go0.0.0", sesudah instal (/usr/local/go/bin/go) → versi yang SALAH
        if path.is_some() {
            Ok("go9.9.9".into())
        } else {
            Ok("go0.0.0".into())
        }
    }
    fn is_root(&self) -> bool {
        true
    }
    fn run_root(&self, _: &str) -> Result<()> {
        Ok(())
    }
}

struct F;
impl Fs for F {
    fn verify_sha256(&self, _: &PathBuf, _: &str) -> Result<()> {
        Ok(())
    }
    fn tmp_path(&self, filename: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/{filename}"))
    }
}

#[test]
fn parse_fallback_both_sides_and_verify_mismatch() {
    // "broken" → memicu unwrap_or(GoSemver{0,0,0}) untuk va/vb di comparator
    let json = r#"
    [
      {"version":"broken","stable":true,"files":[
        {"filename":"go1.2.3.linux-amd64.tar.gz","os":"linux","arch":"amd64","sha256":"deadbeef","kind":"archive","size":null}
      ]},
      {"version":"go1.2.3","stable":true,"files":[
        {"filename":"go1.2.3.linux-amd64.tar.gz","os":"linux","arch":"amd64","sha256":"deadbeef","kind":"archive","size":null}
      ]}
    ]"#.to_string();

    let http = H { json };
    let sys = S;
    let fs = F;

    let err = run_update(&http, &sys, &fs).unwrap_err();
    // baris: return Err(anyhow!("verifikasi gagal: {newv} != {}"
    assert!(err.to_string().contains("verifikasi gagal"));
}
