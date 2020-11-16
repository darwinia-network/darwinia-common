// --- std ---
use std::{env, fs, io::Read, path::Path};

fn main() {
	let mut dags_merkle_roots_file =
		fs::File::open("../../../../bin/pangolin/node/res/dags-merkle-roots.json").unwrap();
	let mut dags_merkle_roots_str = String::new();
	dags_merkle_roots_file
		.read_to_string(&mut dags_merkle_roots_str)
		.unwrap();

	fs::write(
		&Path::new(&env::var_os("OUT_DIR").unwrap()).join("dags_merkle_roots.rs"),
		&format!(
			"pub const DAGS_MERKLE_ROOTS_STR: &'static str = r#\"{}\"#;",
			dags_merkle_roots_str,
		),
	)
	.unwrap();
}
