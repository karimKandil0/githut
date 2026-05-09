{
  description = "githut - GitHub repository discovery TUI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain

            # git2 / libgit2 bindings
            pkg-config
            openssl
            libgit2
            zlib

            # dev tools
            cargo-watch
            cargo-edit
            cargo-expand
          ];

          shellHook = ''
            echo "githut dev shell"
            echo "rust: $(rustc --version)"
            echo "cargo: $(cargo --version)"
          '';

          # needed for git2 / openssl to find libs
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.libgit2}/lib/pkgconfig";
          LIBGIT2_SYS_USE_PKG_CONFIG = "1";
          OPENSSL_NO_VENDOR = "1";
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "githut";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          buildInputs = with pkgs; [ openssl libgit2 zlib ];
          nativeBuildInputs = with pkgs; [ pkg-config ];

          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          LIBGIT2_SYS_USE_PKG_CONFIG = "1";
          OPENSSL_NO_VENDOR = "1";
        };
      });
}
