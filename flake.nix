{
  description = "A Rust library for parsing i18next translation files";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    # Systems supported
    allSystems = [
      "x86_64-linux" # 64-bit Intel/AMD Linux
      "aarch64-linux" # 64-bit ARM Linux
      "x86_64-darwin" # 64-bit Intel macOS
      "aarch64-darwin" # 64-bit ARM macOS
    ];

    # Helper to provide system-specific attributes
    forAllSystems = f:
      nixpkgs.lib.genAttrs allSystems (system:
        f {
          pkgs = import nixpkgs {inherit system;};
        });
  in {
    formatter = forAllSystems ({pkgs}: pkgs.alejandra);

    packages = forAllSystems ({pkgs, ...}: let
      rustPlatform = pkgs.rustPlatform;
      lib = pkgs.lib;
      package = (lib.importTOML ./Cargo.toml).package;
    in {
      default = rustPlatform.buildRustPackage {
        pname = package.name;
        version = package.version;
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };

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
    });
  };
}
