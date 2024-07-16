{
  lib,
  rustPlatform,
  installShellFiles,
  rev ? "dirty",
}: let
  inherit (lib.importTOML ./Cargo.toml) package;
in
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

    nativeBuildInputs = [
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
      inherit (package) description;
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
  }
