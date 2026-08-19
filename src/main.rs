mod code;
mod query;
// mod viewer;

use std::{collections::HashMap, path::PathBuf};

use rustdoc_types::{Function, Item, Crate};

use crate::{query::DLLocations};

const NEEDED_VERSIONS: &[&'static str] = &[
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
    type ItemVec<'a> = Vec<(&'a Item, semver::Version, &'a Crate)>;
    let mut version_functions = HashMap::<String, ItemVec>::new();
    let mut version_modules = HashMap::<String, ItemVec>::new();
    let mut version_structs = HashMap::<String, ItemVec>::new();
    let mut version_enums = HashMap::<String, ItemVec>::new();
    let mut version_traits = HashMap::<String, ItemVec>::new();
    let mut version_aliases = HashMap::<String, ItemVec>::new();
    for (krate, dl) in crates.iter() {
        for (id, item) in krate.index.iter() {
            if let Some(path_item)  = krate.paths.get(id) {
                let full_path = path_item.path.as_slice();
                if full_path.starts_with(&module_path) {
                    let item_path = krate.paths[id].path.join("::");
                    use rustdoc_types::ItemEnum;
                    match &item.inner {
                        ItemEnum::Function(_) => {
                            let list = version_functions.entry(item_path.clone()).or_insert_with(move || Vec::new());
                            list.push((item, dl.version.clone(), krate));
                        },
                        ItemEnum::Module(_) => {
                            let list = version_modules.entry(item_path.clone()).or_insert_with(move || Vec::new());
                            list.push((item, dl.version.clone(), krate));
                        }
                        ItemEnum::Struct(_) => {
                            let list = version_structs.entry(item_path.clone()).or_insert_with(move || Vec::new());
                            list.push((item, dl.version.clone(), krate));
                        }
                        ItemEnum::Enum(_) => {
                            let list = version_enums.entry(item_path.clone()).or_insert_with(move || Vec::new());
                            list.push((item, dl.version.clone(), krate));
                        }
                        ItemEnum::Trait(_) => {
                            let list = version_traits.entry(item_path.clone()).or_insert_with(move || Vec::new());
                            list.push((item, dl.version.clone(), krate));
                        }
                        ItemEnum::TypeAlias(_) => {
                            let list = version_aliases.entry(item_path.clone()).or_insert_with(move || Vec::new());
                            list.push((item, dl.version.clone(), krate));
                        }
                        _ => (),
                    }
                }
            }
        }
    }
    let view_list = [
        ("FUNCTIONS", &version_functions),
        ("MODULES", &version_modules),
        ("STRUCTS", &version_structs),
        ("ENUMS", &version_enums),
        ("TRAITS", &version_traits),
        ("ALIASES", &version_aliases),
    ];
    for (item_type, info) in view_list {
        println!("---- [{item_type}] ----");
        for (path, versions) in info.iter() {
            println!("{path}");
            for (_item, vers, _krate) in versions {
                println!("    - {vers}");
            }
        }
    }
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    download_all()?;
    // QueryApp::create_and_run()?;
    Ok(())
}
