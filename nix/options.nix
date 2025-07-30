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

    desktopEnvironments = mkOption {
      type = types.listOf types.str;
      default = ["common"];
      description = "Lowercase names of the desktop environments to enable the service for";
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
        example = lib.literalExpression
          ''[(lib.getExe pkgs.kitty) "--title" "filepicker"]'';
      };
    };
  };
}
