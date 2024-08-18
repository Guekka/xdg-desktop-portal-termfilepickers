{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.xdg-desktop-portal-termfilepickers;

  inherit (lib.types) types;
  inherit (lib.modules) mkIf;
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
      default = ["gnome" "kde" "xfce" "mate" "lxqt" "lxde" "cinnamon" "pantheon" "budgie" "deepin" "enlightenment" "i3" "sway" "bspwm"];
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
        type = types.str;
        description = "The terminal command to use for opening files";
        example = lib.literalExpression "lib.getExe pkgs.kitty";
      };
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = config.xdg.portal.enable == true;
        message = "xdg.portal must be enabled to use xdg-desktop-portal-termfilepickers";
      }
    ];

    systemd.user.services.xdg-desktop-portal-termfilepickers = let
      configFile = (pkgs.formats.toml {}).generate "config.toml" cfg.config;
    in {
      after = ["graphical-session.target"];
      wantedBy = ["graphical-session.target"];
      serviceConfig = {
        ExecStart = "${lib.getExe cfg.package} --config-path ${configFile}";
        Restart = "on-failure";
      };
    };

    xdg.portal.extraPortals = [cfg.package];

    xdg.portal.config = let
      convert = map (env: {
        name = env;
        value = {"org.freedesktop.impl.portal.FileChooser" = ["termfilepickers"];};
      });
    in
      builtins.listToAttrs (convert cfg.desktopEnvironments);
  };
}
