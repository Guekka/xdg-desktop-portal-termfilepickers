{
  config,
  lib,
  ...
}: let
  cfg = config.services.xdg-desktop-portal-termfilepickers;

  inherit (lib.types) types;
  inherit (lib.options) mkOption mkEnableOption;
in {
  options.services.xdg-desktop-portal-termfilepickers = {
    enable = mkEnableOption "xdg-desktop-portal-termfilepickers";
    package = mkOption {
      type = types.package;
      description = "The xdg-desktop-portal-termfilepickers package";
    };

    systemdTarget = mkOption {
      type = types.str;
      description = "The target that should want the service";
      default = config.wayland.systemd.target;
    };

    desktopEnvironments = mkOption {
      type = types.listOf types.str;
      default = ["common"];
      description = "Lowercase names of the desktop environments to enable the service for";
    };

    setGtkEnvironment = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Set `GTK_USE_PORTAL=1` and `GDK_DEBUG=portals` in the session.

        Without these, GTK applications use their own file chooser and never reach
        the portal, which looks exactly like the portal being broken. GTK 3 reads
        `GTK_USE_PORTAL`, GTK 4 ignores it and reads `GDK_DEBUG=portals` instead, so
        both are needed in practice.

        These affect every GTK application in the session, not just the file picker.
        Firefox-based browsers need `widget.use-xdg-desktop-portal.file-picker` set
        to `1` in `about:config` separately; that cannot be done from here.
      '';
    };

    forcePortalDir = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Build a directory holding every portal backend plus the generated
        `portals.conf`, and point `xdg-desktop-portal` at it with
        `XDG_DESKTOP_PORTAL_DIR`.

        This makes the file picker work with an `xdg-desktop-portal` that was not
        built by Nix, which is the usual case outside NixOS. Such a build looks for
        backends only in its own compile-time directory (before 1.19) or in
        `XDG_DATA_DIRS` (1.19 and later), and in both cases it ignores the
        `NIX_XDG_DESKTOP_PORTAL_DIR` variable that Home Manager sets, since reading
        it is a nixpkgs patch. The result is that the backend is installed but
        invisible, and applications silently fall back to another file picker.

        `XDG_DESKTOP_PORTAL_DIR` is understood by unpatched builds. Note that it
        replaces the whole lookup, for backends *and* for `portals.conf`, which is
        why the generated directory has to contain both.

        Set this to false if you manage the portal directory yourself, or on NixOS
        where the packaged `xdg-desktop-portal` already handles this.
      '';
    };

    extraPortals = mkOption {
      type = types.listOf types.package;
      default = [];
      example = lib.literalExpression "[pkgs.xdg-desktop-portal-gtk]";
      description = ''
        Extra packages whose portal backends are included in the directory built by
        `forcePortalDir`.

        Because `XDG_DESKTOP_PORTAL_DIR` replaces the lookup rather than adding to
        it, every backend you need must be in that one directory. The backends from
        `xdg.portal.extraPortals` are picked up automatically; list anything here
        that is installed by other means.
      '';
    };

    config = {
      open_file_script_path = mkOption {
        type = types.path;
        description = "The path to the script that will be used to open files";
        default = "${cfg.package}/share/wrappers/yazi-open-file.nu";
      };

      save_file_script_path = mkOption {
        type = types.path;
        description = "The path to the script that will be used to save files";
        default = "${cfg.package}/share/wrappers/yazi-save-file.nu";
      };

      save_files_script_path = mkOption {
        type = types.path;
        description = "The path to the script that will be used to save files";
        # this is not a typo, the package does not provide a separate script for saving multiple files
        default = "${cfg.package}/share/wrappers/yazi-save-file.nu";
      };

      terminal_command = mkOption {
        type = types.listOf types.str;
        description = "The terminal command to use for opening files";
        example =
          lib.literalExpression
          ''[(lib.getExe pkgs.kitty) "--title" "filepicker"]'';
      };
    };
  };
}
