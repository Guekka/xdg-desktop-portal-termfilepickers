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

    getBuildInputs = pkgs: with pkgs; [openssl nushell yazi];
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
    packages = pkgsForAllSystems (pkgs: let
      makePackage = {
        customYazi ? null,
        replaceYazi ? true,
      }: let
        yaziPath = if customYazi != null then customYazi else pkgs.yazi;
      in
        pkgs.rustPlatform.buildRustPackage {
          inherit (cargoToml.package) name version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          RUST_BACKTRACE = "full";

          buildInputs = getBuildInputs pkgs;
          nativeBuildInputs = getNativeBuildInputs pkgs;

          postPatch = pkgs.lib.optionalString replaceYazi ''
            substituteInPlace data/share/wrappers/yazi-open-file.nu \
              --replace-fail '"yazi"' '"${yaziPath}/bin/yazi"'
            substituteInPlace data/share/wrappers/yazi-save-file.nu \
              --replace-fail '"yazi"' '"${yaziPath}/bin/yazi"'
          '';

          postInstall = ''
            cp -r data/share $out/share
          '';

          meta = with pkgs.lib; {
            description = "A FileChooser XDG desktop portal allowing to run custom scripts to open and save files";
            mainProgram = "xdg-desktop-portal-termfilepickers";
            license = licenses.mpl20;
            platforms = platforms.all;
          };

          passthru = {
            override = args: makePackage args;
          };
        };
    in {
      default = makePackage {};
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
