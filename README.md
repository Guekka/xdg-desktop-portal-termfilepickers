# XDG Desktop Portal Termfilepickers

This is a desktop portal for file picking and saving, which is used by Flatpak applications.

I have been daily-driving it for the past few months to replace [xdg-desktop-portal-termfilechooser](https://github.com/exquo/xdg-desktop-portal-termfilechooser/)

## Installation

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

## Configuration Options

### Custom Yazi Binary

By default, the package uses `pkgs.yazi` as the file manager. You can customize this by using the `override` function with the `customYazi` argument:

```nix
  services.xdg-desktop-portal-termfilepickers = let
    termfilepickers = inputs.xdp-termfilepickers.packages.${pkgs.system}.default.override {
      customYazi = pkgs.yazi-unwrapped;  # or any other yazi package/path
    };
  in {
    enable = true;
    package = termfilepickers;
    config = {
      terminal_command = [(lib.getExe pkgs.kitty)];
    };
  };
```

### Disabling Yazi Path Replacement

If you want to disable the automatic replacement of `yazi` with the full Nix store path in the wrapper scripts (useful if you want to use yazi from your PATH), you can set `replaceYazi` to `false`:

```nix
  services.xdg-desktop-portal-termfilepickers = let
    termfilepickers = inputs.xdp-termfilepickers.packages.${pkgs.system}.default.override {
      replaceYazi = false;
    };
  in {
    enable = true;
    package = termfilepickers;
    config = {
      terminal_command = [(lib.getExe pkgs.kitty)];
    };
  };
```

You can also combine both options:

```nix
  services.xdg-desktop-portal-termfilepickers = let
    termfilepickers = inputs.xdp-termfilepickers.packages.${pkgs.system}.default.override {
      customYazi = pkgs.yazi-unwrapped;
      replaceYazi = true;
    };
  in {
    enable = true;
    package = termfilepickers;
    config = {
      terminal_command = [(lib.getExe pkgs.kitty)];
    };
  };
```

_This documentation is work in progress_
