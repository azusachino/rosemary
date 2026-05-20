{
  "description": "Rosemary: Rust learning project",

  "inputs": {
    "nixpkgs.url": "github:NixOS/nixpkgs/nixos-unstable";
    "flake-utils.url": "github:numtide/flake-utils";
    "rust-overlay.url": "github:oxalica/rust-overlay";
  };

  "outputs": { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rust
            cargo
            clippy
            rustfmt
            rust-analyzer
            # Common
            nodePackages.prettier
            taplo
            shfmt
            # Extra
            openssl
            pkg-config
          ];

          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
          '';
        };
      });
}
