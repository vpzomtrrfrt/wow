#[macro_use] extern crate serde_derive;
extern crate serde_yaml;
extern crate mkdirp;
extern crate crypto_hash;
extern crate hex_slice;

mod objects;

fn download(href: &str, path: &std::path::Path) {
    if path.exists() {
        println!("Source already downloaded.");
        return;
    }
    let result = std::process::Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(path)
        .arg(href.to_owned())
        .status()
        .unwrap();
    if !result.success() {
        panic!("Failed to download source.");
    }
}

fn validate(verification: &objects::SourceVerification, path: &std::path::Path) -> std::io::Result<bool> {
    match *verification {
        objects::SourceVerification::Sha256(ref s) => {
            let mut hasher = crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA256);
            let mut file = std::fs::File::open(path)?;
            std::io::copy(&mut file, &mut hasher)?;
            use hex_slice::AsHex;
            let hash = format!("{:02x}", hasher.finish().plain_hex(false));
            Ok(hash == s.sum)
        }
    }
}

fn build(spec: &objects::BuildSpec, srcdir: &std::path::Path, pkgdir: &std::path::Path, workdir: &std::path::Path) {
    println!("Starting build");
    let script = &spec.scripts.install;
    let srcdir = srcdir.canonicalize().unwrap();
    let pkgdir = pkgdir.canonicalize().unwrap();
    let workdir = workdir.canonicalize().unwrap();
    let mut child = std::process::Command::new("bash")
        .env("srcdir", &srcdir)
        .env("pkgdir", &pkgdir)
        .env("workdir", &workdir)
        .env("version", &spec.version)
        .current_dir(&workdir)
        .stdin(std::process::Stdio::piped())
        .spawn().unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();

        use std::io::Write;
        stdin.write_all(b"set -e -o pipefail -u\n").unwrap();
        for line in script {
            stdin.write_all(line.as_bytes()).unwrap();
            stdin.write_all(b"\n").unwrap();
        }
    }

    let status = child.wait().unwrap();
    if !status.success() {
        panic!("Failed to build package.");
    }
}

fn main() {
    println!("Hello, world!");
    let spec: objects::BuildSpec = {
        let file = std::fs::File::open("build.yml").unwrap();
        serde_yaml::from_reader(file).unwrap()
    };
    let sources_dir = std::path::PathBuf::from("build/sources");
    let install_dir = std::path::PathBuf::from("build/pkg");
    let work_dir = std::path::PathBuf::from("build/work");
    mkdirp::mkdirp(&sources_dir).unwrap();
    mkdirp::mkdirp(&install_dir).unwrap();
    mkdirp::mkdirp(&work_dir).unwrap();
    for source in &spec.sources {
        mkdirp::mkdirp(&sources_dir.to_owned()).unwrap();
        let filename = {
            match source.href.rfind('/') {
                Some(index) => &source.href[index + 1..],
                None => &source.href
            }
        };
        let target = sources_dir.join(filename);
        download(&source.href, &target);
        if !validate(&source.verification, &target).unwrap() {
            panic!("Source validation failed!");
        }
    }
    build(&spec, &sources_dir, &install_dir, &work_dir);
    println!("{:?}", spec);
}
