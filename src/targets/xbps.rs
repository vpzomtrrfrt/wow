extern crate plist;
extern crate tempfile;
extern crate target;

use std;
use crypto_hash;
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

#[derive(Serialize)]
struct PkgDir {
    file: String
}

#[derive(Serialize)]
struct PkgFile {
    file: String,
    mtime: u64,
    sha256: String
}

#[derive(Serialize)]
struct PkgFiles {
    dirs: Vec<PkgDir>,
    files: Vec<PkgFile>
}

impl PkgFiles {
    fn new() -> Self {
        PkgFiles {
            dirs: vec![],
            files: vec![]
        }
    }
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
        SystemTime(err: std::time::SystemTimeError) {
            from()
        }
        StripPrefix(err: std::path::StripPrefixError) {
            from()
        }
        InvalidFilePath {}
        CommandStatus {}
    }
}

fn hash(path: &std::path::Path) -> std::io::Result<String> {
    let mut hasher = crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA256);
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    use hex_slice::AsHex;
    let hash = format!("{:02x}", hasher.finish().plain_hex(false));
    Ok(hash)
}

fn process_dir(root: &std::path::Path, dir: &std::path::Path, files: &mut PkgFiles, linkdir: Option<&std::path::Path>) -> Result<u64, Error> {
    let mut size = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if let Some(linkdir) = linkdir {
            let path = entry.path();
            let target = linkdir.join(path.file_name().ok_or_else(|| Error::InvalidFilePath)?);
            println!("linking {} in {}", path.display(), target.display());
            std::os::unix::fs::symlink(path, target)?;
        }
        let filetype = entry.file_type()?;
        let path = entry.path();
        if filetype.is_dir() {
            files.dirs.push(PkgDir {
                file: format!("/{}", path.strip_prefix(root)?.display())
            });
            size += process_dir(root, &path, files, None)?;
        }
        else if filetype.is_file() {
            let metadata = entry.metadata()?;
            files.files.push(PkgFile {
                file: format!("/{}", path.strip_prefix(root)?.display()),
                mtime: metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs(),
                sha256: hash(&path)?
            });
            size += metadata.len();
        }
    }
    Ok(size)
}

fn process_files(pkgdir: &std::path::Path, tmpdir: &std::path::Path) -> Result<(u64, PkgFiles), Error> {
    let mut files = PkgFiles::new();
    Ok((process_dir(pkgdir, pkgdir, &mut files, Some(tmpdir))?, files))
}

pub fn package(spec: &objects::BuildSpec, pkgdir: &std::path::Path, destdir: &std::path::Path) -> Result<Box<std::path::PathBuf>, Error> {
    println!("Starting [package]");
    let pkgdir = pkgdir.canonicalize()?;
    let destdir = destdir.canonicalize()?;
    let arch = target::arch();
    let dest = destdir.join(format!("{}-{}_{}.{}.xbps", spec.name, spec.version, spec.epoch, arch));
    {
        let tmpdir = tempfile::tempdir()?;
        let tmpdir = tmpdir.path();
        let (size, files) = process_files(&pkgdir, tmpdir)?;
        let files_file = std::fs::File::create(tmpdir.join("files.plist"))?;
        plist::serde::serialize_to_xml(files_file, &files)?;
        let props = PkgProps {
            architecture: arch.to_owned(),
            installed_size: size,
            pkgname: spec.name.to_owned(),
            pkgver: format!("{}-{}_{}", spec.name, spec.version, spec.epoch),
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
        let props_file = std::fs::File::create(tmpdir.join("props.plist"))?;
        plist::serde::serialize_to_xml(props_file, &props)?;
        println!("Props file created.");
        let result = std::process::Command::new("tar")
            .current_dir(tmpdir)
            .arg("chJf")
            .arg(&dest)
            .arg(".")
            .status()?;
        if !result.success() {
            return Err(Error::CommandStatus);
        }
    }
    Ok(Box::new(dest))
}
