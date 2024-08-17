{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = import inputs.systems;
      imports = [
        inputs.treefmt-nix.flakeModule
      ];
      perSystem = {
        config,
        self',
        pkgs,
        lib,
        system,
        ...
      }: let
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rust-toolchain = pkgs.symlinkJoin {
          name = "rust-toolchain";
          paths = with pkgs; [
            cargo-deny
            cargo-edit
            cargo-watch
            rust-analyzer
            rustc
            cargo
            cargo-watch
            rustPlatform.rustcSrc
          ];
        };

        buildInputs = with pkgs; [openssl nushell];
        nativeBuildInputs = with pkgs; [rustPlatform.bindgenHook pkg-config];
      in {
        # Rust package
        packages.default = pkgs.rustPlatform.buildRustPackage {
          inherit (cargoToml.package) name version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          RUST_BACKTRACE = "full";

          buildInputs = buildInputs;
          nativeBuildInputs = nativeBuildInputs;

          postInstall = ''
            cp -r data/share $out/share
          '';
        };

        # Rust dev environment
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            config.treefmt.build.devShell
          ];
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          nativeBuildInputs = nativeBuildInputs;
          packages = buildInputs ++ [rust-toolchain pkgs.clippy];
        };

        treefmt.config = {
          projectRootFile = "flake.nix";
          programs = {
            alejandra.enable = true;
            rustfmt.enable = true;
          };
        };
      };
    };
}
