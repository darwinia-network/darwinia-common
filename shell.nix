let
  rust-overlay =
    import (builtins.fetchGit {
      url = "https://github.com/oxalica/rust-overlay.git";
      rev = "84c58400556c1c5fa796cbc3215ba5bbd3bd848f";
    });
  nixpkgs = import <nixpkgs> { overlays = [ rust-overlay ]; };
  rust-nightly = with nixpkgs; (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
    extensions = [ "rust-src" ];
    targets = [ "wasm32-unknown-unknown" ];
  }));
in
with nixpkgs; pkgs.mkShell {
  nativeBuildInputs = [
	rust-nightly
  ];

  buildInputs = [
	rocksdb
	clang
  ];

  RUST_SRC_PATH="${rust-nightly}/lib/rustlib/src/rust/src";
  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  PROTOC = "${protobuf}/bin/protoc";
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
}
