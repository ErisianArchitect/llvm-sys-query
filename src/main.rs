mod code;
mod query;
mod viewer;

use std::{collections::HashMap, path::PathBuf};

use rustdoc_types::{Function, Item, Crate};

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
    let mut version_functions = HashMap::<String, Vec<(&Item, semver::Version, &Crate)>>::new();
    'crates_iter: for (krate, dl) in crates.iter() {
        for (id, item) in krate.index.iter() {
            if let Some(path_item)  = krate.paths.get(id) {
                let full_path = path_item.path.as_slice();
                if full_path.starts_with(&module_path) {
                    let item_path = krate.paths[id].path.join("::");
                    match &item.inner {
                        rustdoc_types::ItemEnum::Function(function) => {
                            let list = version_functions.entry(item_path.clone()).or_insert_with(move || Vec::new());
                            list.push((item, dl.version.clone(), krate));
                            // if let Some(span) = item.span.as_ref() {
                            //     let source_path = dl.source.join(&span.filename);
                            //     println!("--- {item_path}");
                            //     println!("\x1b[38;2;0;150;230m{}:{}:{}\x1b[39m", source_path.display(), span.begin.0, span.begin.1);
                            //     let source_code = code::query_code(&dl.source, span)?;
                            //     println!("\x1b[38;2;0;200;100m{source_code}\x1b[39m");
                            // }
                        },
                        _ => (),
                    }
                }
            }
        }
    }
    for (path, versions) in version_functions.iter() {
        // println!("\x1b[38;2;0;150;250m{path}\x1b[39m");
        println!("{path}");
        for (_item, vers, krate) in versions {
            // match _item.inner.item_kind() {
            //     rustdoc_types::ItemKind::Module => todo!(),
            //     rustdoc_types::ItemKind::ExternCrate => todo!(),
            //     rustdoc_types::ItemKind::Use => todo!(),
            //     rustdoc_types::ItemKind::Struct => todo!(),
            //     rustdoc_types::ItemKind::StructField => todo!(),
            //     rustdoc_types::ItemKind::Union => todo!(),
            //     rustdoc_types::ItemKind::Enum => todo!(),
            //     rustdoc_types::ItemKind::Variant => todo!(),
            //     rustdoc_types::ItemKind::Function => todo!(),
            //     rustdoc_types::ItemKind::TypeAlias => todo!(),
            //     rustdoc_types::ItemKind::Constant => todo!(),
            //     rustdoc_types::ItemKind::Trait => todo!(),
            //     rustdoc_types::ItemKind::TraitAlias => todo!(),
            //     rustdoc_types::ItemKind::Impl => todo!(),
            //     rustdoc_types::ItemKind::Static => todo!(),
            //     rustdoc_types::ItemKind::ExternType => todo!(),
            //     rustdoc_types::ItemKind::Macro => todo!(),
            //     rustdoc_types::ItemKind::ProcAttribute => todo!(),
            //     rustdoc_types::ItemKind::ProcDerive => todo!(),
            //     rustdoc_types::ItemKind::AssocConst => todo!(),
            //     rustdoc_types::ItemKind::AssocType => todo!(),
            //     rustdoc_types::ItemKind::Primitive => todo!(),
            //     rustdoc_types::ItemKind::Keyword => todo!(),
            //     rustdoc_types::ItemKind::Attribute => todo!(),
            // }
            println!("- {vers}");
        }
    }
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    download_all()?;
    // QueryApp::create_and_run()?;
    Ok(())
}
