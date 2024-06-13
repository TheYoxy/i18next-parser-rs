{
  description = "Rust example flake for Zero to Nix";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    # Provides helpers for Rust toolchains
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
       # Overlays enable you to customize the Nixpkgs attribute set
        overlays = [
        # Makes a `rust-bin` attribute available in Nixpkgs
        (import rust-overlay)
        # Provides a `rustToolchain` attribute for Nixpkgs that we can use to
        # create a Rust environment
        (self: super: {
          rustToolchain = super.rust-bin.stable.latest.default;
        })
      ];

      # Systems supported
      allSystems = [
        "x86_64-linux" # 64-bit Intel/AMD Linux
        "aarch64-linux" # 64-bit ARM Linux
        "x86_64-darwin" # 64-bit Intel macOS
        "aarch64-darwin" # 64-bit ARM macOS
      ];

      # Helper to provide system-specific attributes
      forAllSystems = f: nixpkgs.lib.genAttrs allSystems (system: f {
        pkgs = import nixpkgs { inherit overlays system; };
      });
    in
    {
      overlays.default = final: prev: {
        # The Rust toolchain used for the package build
        rustToolchain = final.rust-bin.stable.latest.default;
      };

      # Development environment output
      devShells = forAllSystems ({ pkgs }: {
        default = pkgs.mkShell {
          # The Nix packages provided in the environment
          packages = (with pkgs; [
            # The package provided by our custom overlay. Includes cargo, Clippy, cargo-fmt,
            # rustdoc, rustfmt, and other tools.
            rustToolchain
          ]) ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs; [ libiconv ]);
        };
      });

      packages = forAllSystems ({ pkgs }: {
        default =
          let
            rustPlatform = pkgs.makeRustPlatform {
              cargo = pkgs.rustToolchain;
              rustc = pkgs.rustToolchain;
            };
          in
          rustPlatform.buildRustPackage {
            name = "i18next-parser-rs";
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
          };
      });
    };
}
