{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.services.xdg-desktop-portal-termfilepickers;
in {
  imports = [./options.nix];

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = config.xdg.portal.enable == true;
        message = "xdg.portal must be enabled to use xdg-desktop-portal-termfilepickers";
      }
    ];

    systemd.user.services.xdg-desktop-portal-termfilepickers = let
      configFile = (pkgs.formats.toml {}).generate "config.toml" cfg.config;
    in {
      Unit = {
        Description = "Portal service (termfilepickers implementation)";
        PartOf = ["graphical-session.target"];
        After = ["graphical-session.target"];
      };

      Service = {
        Type = "dbus";
        BusName = "org.freedesktop.impl.portal.desktop.termfilepickers";
        ExecStart = "${lib.getExe cfg.package} --config-path ${configFile}";
        Restart = "on-failure";
      };

      Install = {
        WantedBy = ["graphical-session.target"];
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
