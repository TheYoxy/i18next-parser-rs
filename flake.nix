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
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }: let
    overlays = [
      rust-overlay.overlays.default
      (final: prev: {
        rustToolchain = final.rust-bin.stable.latest.default.override {
          extensions = ["rust-src"];
        };
      })
    ];
  in
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system overlays;};
    in {
      formatter = pkgs.alejandra;

      packages = let
        lib = pkgs.lib;
        package = (lib.importTOML ./Cargo.toml).package;
      in {
        default =
          (pkgs.makeRustPlatform {
            cargo = pkgs.rustToolchain;
            rustc = pkgs.rustToolchain;
          })
          .buildRustPackage {
            pname = package.name;
            version = package.version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            doCheck = false;
            meta = {
              description = package.description;
              homepage = package.repository;
              license = lib.licenses.mit;
              maintainers = [
                {
                  name = "TheYoxy";
                  email = "floryansimar@gmail.com";
                }
              ];
            };
          };
      };
    });
}
