{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.services.xdg-desktop-portal-termfilepickers;

  portalConfig = let
    convert = map (env: {
      name = env;
      value = {"org.freedesktop.impl.portal.FileChooser" = ["termfilepickers"];};
    });
  in
    builtins.listToAttrs (convert cfg.desktopEnvironments);

  portalDir = import ./portal-dir.nix {
    inherit pkgs lib;
    portals = config.xdg.portal.extraPortals ++ cfg.extraPortals;
    config = portalConfig;
  };

  # GTK 3 reads GTK_USE_PORTAL, GTK 4 only honours GDK_DEBUG=portals
  gtkEnvironment = {
    GTK_USE_PORTAL = "1";
    GDK_DEBUG = "portals";
  };
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
      Unit =
        {
          After = [cfg.systemdTarget];
          PartOf = [cfg.systemdTarget];
        }
        // lib.optionalAttrs (!config.xsession.enable) {
          ConditionEnvironment = "WAYLAND_DISPLAY";
        };

      Service = {
        ExecStart = "${lib.getExe cfg.package} --config-path ${configFile}";
        Restart = "on-failure";
      };

      Install = {
        WantedBy = [cfg.systemdTarget];
      };
    };

    xdg.portal.extraPortals = [cfg.package];

    xdg.portal.config = portalConfig;

    # An xdg-desktop-portal that Nix did not build ignores
    # NIX_XDG_DESKTOP_PORTAL_DIR, so the backend installed above is invisible to it
    # and applications fall back to another file picker. XDG_DESKTOP_PORTAL_DIR is
    # understood by those builds too.
    home.sessionVariables = lib.mkMerge [
      (lib.mkIf cfg.forcePortalDir {XDG_DESKTOP_PORTAL_DIR = "${portalDir}";})
      (lib.mkIf cfg.setGtkEnvironment gtkEnvironment)
    ];

    systemd.user.sessionVariables = lib.mkMerge [
      (lib.mkIf cfg.forcePortalDir {XDG_DESKTOP_PORTAL_DIR = "${portalDir}";})
      (lib.mkIf cfg.setGtkEnvironment gtkEnvironment)
    ];
  };
}
