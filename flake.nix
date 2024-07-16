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
      (final: prev: {
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
              rustToolchain
            ];
          };
      };

      packages = let
        inherit (pkgs) lib;
        inherit (lib.importTOML ./Cargo.toml) package;
        rev = self.shortRev or self.dirtyShortRev or "dirty";
      in rec {
        i18next-parser =
          rustPlatform
          .buildRustPackage {
            pname = package.name;
            version = "${package.version}-${rev}";
            src = lib.fileset.toSource {
              root = ./.;
              fileset =
                lib.fileset.intersection
                (lib.fileset.fromSource (lib.sources.cleanSource ./.))
                (lib.fileset.unions [
                  ./crates
                  ./Cargo.toml
                  ./Cargo.lock
                  ./build.rs
                ]);
            };
            cargoLock.lockFile = ./Cargo.lock;

            doCheck = false;

            nativeBuildInputs = with pkgs; [
              installShellFiles
            ];

            preFixup = ''
              mkdir completions

              $out/bin/${package.name} --generate-shell bash > completions/${package.name}.bash
              $out/bin/${package.name} --generate-shell zsh > completions/${package.name}.zsh
              $out/bin/${package.name} --generate-shell fish > completions/${package.name}.fish

              installShellCompletion completions/*
            '';
            meta = {
              description = package.description;
              homepage = package.repository;
              license = lib.licenses.mit;
              mainProgram = package.name;
              maintainers = [
                {
                  name = "TheYoxy";
                  email = "floryansimar@gmail.com";
                }
              ];
            };
          };
        default = i18next-parser;
      };
    });
}
