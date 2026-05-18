{
  description = "A Rust devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    (flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        packages.default = pkgs.callPackage ./nix/package.nix { };

        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              rust
              bun
              pkg-config
              cargo-tarpaulin
              librsvg
              webkitgtk_4_1
              (pkgs.writeShellScriptBin "cargoo" ''
                subcommand="$1"
                shift
                exec cargo "$subcommand" --manifest-path src-tauri/Cargo.toml "$@"
              '')
            ];
            
            shellHook = ''
              # Needed on Wayland to report the correct display scale
              export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH"
            '';
          };
      }
    ))
    // {
      overlays.default = final: _prev: {
        oxeylyzer = final.callPackage ./nix/package.nix { };
      };
    };
}
