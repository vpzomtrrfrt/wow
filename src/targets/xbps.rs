extern crate plist;
extern crate tempfile;
extern crate target;

use std;
use objects;

#[derive(Serialize)]
struct PkgProps {
    architecture: String,
    installed_size: u64,
    pkgname: String,
    pkgver: String,
    run_depends: Vec<String>,
    version: String,
    short_desc: Option<String>,
    homepage: Option<String>,
    license: Option<String>,
    maintainer: Option<String>
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: std::io::Error) {
            from()
        }
        Plist(err: plist::Error) {
            from()
        }
        InvalidFilePath {}
        CommandStatus {}
    }
}

pub fn package(spec: &objects::BuildSpec, pkgdir: &std::path::Path, destdir: &std::path::Path) -> Result<Box<std::path::PathBuf>, Error> {
    println!("Starting [package]");
    let pkgdir = pkgdir.canonicalize()?;
    let destdir = destdir.canonicalize()?;
    let arch = target::arch();
    let dest = destdir.join(format!("{}-{}_{}.{}.xbps", spec.name, spec.version, 1, arch));
    let props = PkgProps {
        architecture: arch.to_owned(),
        installed_size: std::fs::metadata(&pkgdir)?.len(),
        pkgname: spec.name.to_owned(),
        pkgver: format!("{}-{}", spec.name, spec.version),
        run_depends: [&spec.depends.all, &spec.depends.run]
            .iter()
            .flat_map(|l| l.iter())
            .map(|s| s.to_owned())
            .collect(),
        version: spec.version.to_owned(),
        short_desc: None,
        homepage: None,
        license: None,
        maintainer: None
    };
    {
        let tmpdir = tempfile::tempdir()?;
        let tmpdir = tmpdir.path();
        println!("exists? {}", tmpdir.exists());
        let props_file = std::fs::File::create(tmpdir.join("props.plist"))?;
        plist::serde::serialize_to_xml(props_file, &props)?;
        println!("Props file created.");
        for entry in std::fs::read_dir(pkgdir)? {
            let entry = entry?.path();
            println!("exists2? {}", entry.exists());
            let target = tmpdir.join(entry.file_name().ok_or_else(|| Error::InvalidFilePath)?);
            println!("linking {} in {}", entry.display(), target.display());
            std::os::unix::fs::symlink(entry, target)?;
        }
        let result = std::process::Command::new("tar")
            .current_dir(tmpdir)
            .arg("cf")
            .arg(&dest)
            .arg(".")
            .status()?;
        if !result.success() {
            return Err(Error::CommandStatus);
        }
    }
    Ok(Box::new(dest))
}
