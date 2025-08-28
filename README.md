# XDG Desktop Portal Termfilepickers

This is a desktop portal for file picking and saving, which is used by Flatpak applications.

I have been daily-driving it for the past few months to replace [xdg-desktop-portal-termfilechooser](https://github.com/exquo/xdg-desktop-portal-termfilechooser/)

To use it, the NixOS / HM modules are recommended. For example:
```nix
  imports = [inputs.xdp-termfilepickers.homeManagerModules.default];

  services.xdg-desktop-portal-termfilepickers = let
    termfilepickers = inputs.xdp-termfilepickers.packages.${pkgs.system}.default;
  in {
    enable = true;
    package = termfilepickers;
    config = {
      terminal_command = [(lib.getExe pkgs.kitty)];
    };
  };
```

_This documentation is work in progress_
