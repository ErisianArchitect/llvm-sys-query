use std::{path::{Path, PathBuf}, sync::LazyLock, time::Duration};
use crates_index::{
    GitIndex,
    Crate,
    Error as CratesIndexError,
    Version,
};
use flate2::{Compression, read::GzDecoder};
pub use semver;

use semver::{
    Version as Semver,
    Prerelease,
    VersionReq,
    BuildMetadata,
    Comparator,
    Error as SemverError,
};
use sha2::Digest;

#[derive(Debug, Clone, Copy)]
struct UserAgent(&'static str);

const USER_AGENT: UserAgent = UserAgent(uranus::package_version!("github.com/ErisianArchitect/llvm-sys-query@{full}"));

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}")]
    CratesIndex(#[from] CratesIndexError),
    #[error("{0}")]
    Semver(#[from] semver::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    RustdocBuildError(#[from] rustdoc_json::BuildError),
    #[error("Crate not found")]
    CrateNotFound,
    #[error("Crate with that version was not found")]
    VersionNotFound,
    #[error("Crate download URL not found")]
    NoDownloadUrl,
    #[error("Checksums did not match")]
    ChecksumMismatch,
}

pub fn get_hash(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

pub fn get_crate(krate: &str) -> Result<Crate, QueryError> {
    let index = GitIndex::new_cargo_default()?;
    let Some(krate) = index.crate_(krate) else {
        return Err(QueryError::CrateNotFound);
    };
    Ok(krate)
}

pub fn download_crate<P: AsRef<Path>>(
    krate: &str,
    version: Option<&Semver>,
    path: P,
) -> Result<Crate, QueryError> {
    fn download_crate(
        krate: &str,
        version: Option<&Semver>,
        path: &Path
    ) -> Result<Crate, QueryError> {
        let index = GitIndex::new_cargo_default()?;
        let config = index.index_config()?;
        let Some(krate) = index.crate_(krate) else {
            return Err(QueryError::CrateNotFound);
        };
        let version = if let Some(version) = version {
            'get_version: {
                for vers in krate.versions() {
                    let vers_semver = Semver::parse(vers.version())?;
                    if &vers_semver == version {
                        break 'get_version vers;
                    }
                }
                return Err(QueryError::VersionNotFound);
            }
        } else {
            krate.highest_version()
        };

        let Some(dl_url) = version.download_url(&config) else {
            return Err(QueryError::NoDownloadUrl);
        };

        let req = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT.0)
            .build()?;

        let resp = req.get(&dl_url)
            .send()?;

        let bytes = resp.bytes()?;

        let hash = get_hash(&bytes);

        if &hash != version.checksum() {
            return Err(QueryError::ChecksumMismatch);
        }
        
        let mut decoder = GzDecoder::new(&bytes as &[u8]);
        let mut archive = tar::Archive::new(&mut decoder);
        
        for entry in archive.entries()? {
            let mut entry = entry?;
            entry.unpack_in(path)?;
        }
        
        Ok(krate)
    }
    download_crate(krate, version, path.as_ref())
}

pub fn get_versions(krate: &str) -> Result<Vec<Version>, QueryError> {
    let krate = get_crate(krate)?;
    Ok(krate.versions().into())
}

pub struct DLLocations {
    pub version: semver::Version,
    pub source: PathBuf,
    pub rustdoc: PathBuf,
}

pub fn download_and_build_crate_rustdoc_json<P: AsRef<Path>>(krate: &str, version: &semver::Version, output_dir: P) -> Result<DLLocations, QueryError> {
    fn inner(krate: &str, version: &semver::Version, output_dir: &Path) -> Result<DLLocations, QueryError> {
        
        let crate_dir = output_dir.join(krate);
        
        let source_dir = crate_dir.join("source");
        let versions_dir = crate_dir.join("versions");
        let target_dir = crate_dir.join("target");

        std::fs::create_dir_all(&source_dir)?;
        std::fs::create_dir_all(&versions_dir)?;
        std::fs::create_dir_all(&target_dir)?;

        let crate_identifier = format!("{krate}-{version}");
        
        let source_location = source_dir.join(&crate_identifier);
        let manifest_path = source_location.join("Cargo.toml");

        download_crate(krate, Some(version), &source_dir)?;

        // Build the rustdoc json
        let json_path = rustdoc_json::Builder::default()
            .toolchain("nightly")
            .manifest_path(&manifest_path)
            .target_dir(&target_dir)
            .build()?;

        let version_json_name = format!("{version}.json");
        let version_json_path = versions_dir.join(&version_json_name);

        // move json file from target directory into versions directory so the target directory can be removed.
        std::fs::rename(&json_path, &version_json_path)?;

        // remove target directory
        std::fs::remove_dir_all(target_dir)?;
        
        Ok(DLLocations {
            version: version.clone(),
            source: source_location,
            rustdoc: version_json_path,
        })
    }
    inner(krate, version, output_dir.as_ref())
}
