// --- std ---
use std::{env, process};
// --- crates.io ---
use cargo_toml::Manifest;
use walkdir::WalkDir;

fn main() {
	let mut incomplete_dependencies = vec![];

	for e in WalkDir::new(env::current_dir().unwrap())
		.into_iter()
		.filter_entry(|e| {
			let n = e.file_name().to_str().unwrap();

			n != "target" && !(n.starts_with('.') && !n.starts_with("./"))
		})
		.filter_map(|e| e.ok())
	{
		if e.file_name() == "Cargo.toml" {
			let manifest = Manifest::from_path(e.path()).unwrap();

			if let Some(std) = manifest.features.get("std") {
				for (alias, dependency) in manifest.dependencies.iter() {
					if let Some(detail) = dependency.detail() {
						if let Some(default_features) = detail.default_features {
							if !default_features {
								if !std.contains(&format!("{}/std", alias)) {
									incomplete_dependencies.push((
										alias.to_owned(),
										e.path()
											.to_str()
											.unwrap()
											.split("common/")
											.last()
											.unwrap()
											.to_owned(),
									));
								}
							}
						}
					}
				}
			}
		}
	}

	if !incomplete_dependencies.is_empty() {
		for (alias, path) in incomplete_dependencies {
			println!("Incomplete std feature found for `{}` at `{}`", alias, path);
		}

		process::exit(1);
	}
}
