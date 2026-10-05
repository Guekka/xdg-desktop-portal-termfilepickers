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

## Using it outside NixOS

The Home Manager module works on a distribution whose `xdg-desktop-portal` was not
built by Nix, with no extra configuration. Two things are handled for you.

`forcePortalDir` (default `true`) builds a directory holding every portal backend
plus the generated `portals.conf`, and points the portal at it with
`XDG_DESKTOP_PORTAL_DIR`. This is needed because Home Manager normally advertises its
backends through `NIX_XDG_DESKTOP_PORTAL_DIR`, which only a nixpkgs-patched
`xdg-desktop-portal` reads. A distribution build ignores it and finds nothing, so the
backend is installed yet invisible. Note that `XDG_DESKTOP_PORTAL_DIR` *replaces* the
lookup rather than adding to it, for backends and for `portals.conf` alike, which is
why everything has to be gathered into one directory. If a backend installed outside
Nix goes missing as a result, add its package to `extraPortals`.

`setGtkEnvironment` (default `true`) sets `GTK_USE_PORTAL=1` and `GDK_DEBUG=portals`,
without which GTK applications never call the portal at all. It applies to every GTK
application in the session, so turn it off if that is too broad.

Neither option does anything on NixOS, where the packaged `xdg-desktop-portal` and
the `xdg.portal` module already handle this.

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
