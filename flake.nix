{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = {
    self,
    nixpkgs,
    treefmt-nix,
    ...
  }: let
    forAllSystems = nixpkgs.lib.genAttrs [
      "x86_64-linux"
      "aarch64-linux"
    ];

    pkgsForAllSystems = function:
      forAllSystems (system: function (import nixpkgs {inherit system;}));

    cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    getRustToolchain = pkgs:
      pkgs.symlinkJoin {
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

    getBuildInputs = pkgs: with pkgs; [openssl nushell];
    getNativeBuildInputs = pkgs: with pkgs; [rustPlatform.bindgenHook pkg-config];

    treefmtEval = pkgs:
      treefmt-nix.lib.evalModule pkgs {
        projectRootFile = "flake.nix";
        programs = {
          alejandra.enable = true;
          rustfmt.enable = true;
        };
      };
  in {
    nixosModules = {
      default = self.nixosModules.xdg-desktop-portal-filepickers;
      xdg-desktop-portal-filepickers = import ./nix/nixos.nix;
    };

    homeManagerModules = {
      default = self.homeManagerModules.xdg-desktop-portal-filepickers;
      xdg-desktop-portal-filepickers = import ./nix/hm.nix;
    };

    # Rust package
    packages = pkgsForAllSystems (pkgs: {
      default = pkgs.rustPlatform.buildRustPackage {
        inherit (cargoToml.package) name version;
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        RUST_BACKTRACE = "full";

        buildInputs = getBuildInputs pkgs;
        nativeBuildInputs = getNativeBuildInputs pkgs ++ [pkgs.makeWrapper];

        postInstall = ''
          cp -r data/share $out/share
          
          # Fix nushell shebang in wrapper scripts to use Nix store path
          for script in $out/share/wrappers/*.nu; do
            substituteInPlace "$script" \
              --replace-fail '#!/usr/bin/env nu' '#!${pkgs.nushell}/bin/nu'
          done
          
          # Install DBus service file
          mkdir -p $out/share/dbus-1/services
          sed "s|@LIBEXECDIR@|$out/bin|g" data/org.freedesktop.impl.portal.desktop.termfilepickers.service \
            > $out/share/dbus-1/services/org.freedesktop.impl.portal.desktop.termfilepickers.service
        '';

        meta = with pkgs.lib; {
          description = "A FileChooser XDG desktop portal allowing to run custom scripts to open and save files";
          mainProgram = "xdg-desktop-portal-termfilepickers";
          license = licenses.mpl20;
          platforms = platforms.all;
        };
      };
    });

    # Rust dev environment
    devShells = pkgsForAllSystems (pkgs: {
      default = pkgs.mkShell {
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

        nativeBuildInputs = getNativeBuildInputs pkgs;
        packages = (getBuildInputs pkgs) ++ [(getRustToolchain pkgs) pkgs.clippy];
      };
    });

    formatter = pkgsForAllSystems (pkgs: (treefmtEval pkgs).config.build.wrapper);

    checks = pkgsForAllSystems (pkgs: {
      formatting = (treefmtEval pkgs).config.build.check self;
    });
  };
}
