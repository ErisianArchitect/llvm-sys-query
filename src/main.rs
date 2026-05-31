mod code;
mod query;
mod viewer;

use std::path::PathBuf;

use crate::{query::DLLocations, viewer::app::QueryApp};

const NEEDED_VERSIONS: &[&'static str] = &[
    "110.0.4",
    "120.3.2",
    "130.1.2",
    "140.1.3",
    "150.2.1",
    "160.2.1",
    "170.2.0",
    "181.2.0",
    "191.0.0",
    "201.0.0",
    "211.0.0",
    "221.0.0",
];

fn download_all() -> color_eyre::Result<()> {
    let mut dls = Vec::with_capacity(NEEDED_VERSIONS.len());
    for needed in NEEDED_VERSIONS {
        let ver = semver::Version::parse(needed)?;
        let crate_name = "llvm-sys";
        let json_path = PathBuf::from(format!("./output/{crate_name}/versions/{ver}.json"));
        if json_path.exists() {
            let source_dir = PathBuf::from(format!("./output/{crate_name}/source"));
            let source_path = source_dir.join(format!("{crate_name}-{ver}"));
            if !source_path.exists() {
                query::download_crate(crate_name, Some(&ver), &source_dir)?;
            }
            dls.push(DLLocations {
                rustdoc: json_path,
                source: source_path,
                version: ver,
            });
            continue;
        }
        dls.push(query::download_and_build_crate_rustdoc_json(crate_name, &ver, "./output")?);
    }
    let mut crates = Vec::with_capacity(dls.len());
    for dl in dls {
        use rustdoc_types::{
            Crate,
        };
        use std::{
            io::BufReader,
            fs::File,
        };
        let json_file = File::open(&dl.rustdoc)?;
        let json_file_buffer = BufReader::new(json_file);
        let krate: Crate = serde_json::from_reader(json_file_buffer)?;
        crates.push((krate, dl));
    }
    let module_path = [
        String::from("llvm_sys"),
        String::from("orc2"),
    ];
    'crates_iter: for (krate, dl) in crates.iter() {
        for (id, item) in krate.index.iter() {
            if let Some(path_item)  = krate.paths.get(id) {
                let full_path = path_item.path.as_slice();
                if full_path.starts_with(&module_path) {
                    let item_path = krate.paths[id].path.join("::");
                    match &item.inner {
                        rustdoc_types::ItemEnum::Function(function) => {
                            if let Some(span) = item.span.as_ref() {
                                let source_path = dl.source.join(&span.filename);
                                println!("--- {item_path}");
                                println!("\x1b[38;2;0;150;230m{}:{}:{}\x1b[39m", source_path.display(), span.begin.0, span.begin.1);
                                let source_code = code::query_code(&dl.source, span)?;
                                println!("\x1b[38;2;0;200;100m{source_code}\x1b[39m");
                            }
                            continue;
                        },
                        _ => (),
                    }
                }
            }
        }
        break;
    }
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    QueryApp::create_and_run()?;
    Ok(())
}
