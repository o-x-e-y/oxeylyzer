{
  description = "A Rust devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ] (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: {
        default = pkgs.callPackage ./nix/package.nix { };
      });

      devShells = forAllSystems (
        pkgs':
        let
          pkgs = pkgs'.extend (import rust-overlay);
          rust = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          default =
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
      );

      overlays.default = final: _prev: {
        oxeylyzer = final.callPackage ./nix/package.nix { };
      };
    };
}
