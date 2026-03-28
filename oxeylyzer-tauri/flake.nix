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
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              rust
              bun
              pkg-config
              librsvg
              webkitgtk_4_1
              (pkgs.writeShellScriptBin "" ''
                
              '')
            ];

            shellHook = ''
              # Needed on Wayland to report the correct display scale
              export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH"
            '';
          };
      }
    );
}
