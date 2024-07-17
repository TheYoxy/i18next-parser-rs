{
  description = "A Rust library for parsing i18next translation files";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }: let
    overlays = [
      rust-overlay.overlays.default
      (final: _prev: {
        rustToolchain = final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      })
    ];
  in
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system overlays;};
      rustPlatform = pkgs.makeRustPlatform {
        cargo = pkgs.rustToolchain;
        rustc = pkgs.rustToolchain;
      };
    in {
      formatter = pkgs.alejandra;

      devShells = {
        default = with pkgs;
          mkShell {
            buildInputs = [
              pkg-config
#              rustToolchain
            ];
          };
      };

      packages = let
        rev = self.shortRev or self.dirtyShortRev or "dirty";
      in rec {
        i18next-parser = pkgs.callPackage ./packages.nix {
          inherit rev rustPlatform;
        };
        default = i18next-parser;
      };
    });
}
