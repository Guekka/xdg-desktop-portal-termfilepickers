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
      description = "Portal service (termfilepickers implementation)";
      partOf = ["graphical-session.target"];
      after = ["graphical-session.target"];
      wantedBy = ["graphical-session.target"];
      serviceConfig = {
        Type = "dbus";
        BusName = "org.freedesktop.impl.portal.desktop.termfilepickers";
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

    # Ensure DBus service file is available for system-wide DBus
    environment.pathsToLink = lib.mkAfter [
      "/share/dbus-1/services"
    ];
  };
}
