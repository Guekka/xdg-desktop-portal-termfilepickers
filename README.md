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

## Troubleshooting

If applications keep opening their own file picker, run the built-in diagnostics from
inside your desktop session:

```sh
xdg-desktop-portal-termfilepickers doctor
```

> [!WARNING]
> **`doctor` output is a hint, not ground truth. Read this before acting on it.**
>
> - **It duplicates logic owned by xdg-desktop-portal.** When `doctor` cannot ask the
>   running portal, it falls back to its own reimplementation of the backend selection
>   rules, ported from a specific xdg-desktop-portal version. That reimplementation
>   *will* drift as upstream changes, and the report gives no warning when it has: it
>   states a confident verdict either way. Upstream has already renamed the relevant
>   source file twice in three releases.
> - **The parts that ask the portal directly parse debug logs.** Those log strings are
>   `g_debug` output with no stability guarantee, and their wording changed in every
>   recent release. Unrecognised output makes `doctor` fall back silently.
> - **It encodes assumptions that may not hold on your system**, such as
>   `xdg-desktop-portal` being built with `/etc` and `/usr/share` as its directories.
>   These cannot be detected reliably and are wrong on some distributions.
> - **It was written largely by an LLM**, including the ported selection logic and the
>   remediation hints, and has been tested on a small number of setups. Treat specific
>   claims with suspicion, particularly version cutoffs and suggested fixes.
>
> A `[fail]` is a good place to start looking, and the file paths and backend names it
> reports are usually accurate. Do not treat a clean report as proof the setup is
> correct, and check any command it suggests before running it, especially ones that
> replace distribution files or restart services. The authoritative sources are the
> [portals.conf documentation](https://flatpak.github.io/xdg-desktop-portal/docs/portals.conf.html)
> and `xdg-desktop-portal -vr` output.

It checks the whole chain rather than just this service: the config file and the
scripts it points to, the installed `*.portal` backends, which backend
`xdg-desktop-portal` actually selects for `FileChooser`, whether both services are on
the session bus, and the toolkit settings that decide whether an application talks to
the portal at all. Every problem comes with a hint on how to fix it, and the command
exits non-zero when something is broken.

For the backend selection, `doctor` asks the running `xdg-desktop-portal` directly
instead of guessing: it finds the binary that owns the portal bus name and has it
report its own verdict. That matters because several versions can be installed side
by side, and versions before 1.19 only look for `*.portal` files in their build-time
directory, ignoring `XDG_DATA_DIRS`. A correctly installed backend can be invisible
to them:

```
  [fail] backend visible to xdg-desktop-portal: no
         our backend is installed at /home/user/.nix-profile/share/xdg-desktop-portal/portals/termfilepickers.portal
         but xdg-desktop-portal only scanned: /usr/share/xdg-desktop-portal/portals
         xdg-desktop-portal 1.18.4 only looks in its build-time directory and ignores XDG_DATA_DIRS
         -> XDG_DATA_DIRS support for backend files was added in 1.19
```

When a newer portal is installed but not the one running, `doctor` says why and how
to fix it. This happens on distributions that ship an old `xdg-desktop-portal`
alongside a Nix or Home Manager setup: the portal is D-Bus activated and delegates to
`xdg-desktop-portal.service`, but systemd reads user units only from
`~/.config/systemd/user` and the system directories, never from the Nix profile, so
the distribution unit wins and the newer binary never starts.

A unit in `~/.config/systemd/user` takes precedence over the distribution one, so
defining the service through Home Manager replaces it:

```nix
  systemd.user.services.xdg-desktop-portal = {
    Unit.Description = "Portal service";
    Service = {
      Type = "dbus";
      BusName = "org.freedesktop.portal.Desktop";
      ExecStart = "${pkgs.xdg-desktop-portal}/libexec/xdg-desktop-portal";
    };
  };
```

Then `systemctl --user daemon-reload`, restart the service, and log out and back in.
Replacing a distribution service this way is intrusive: it opts you out of the
distribution's updates to that unit. Upgrading `xdg-desktop-portal` past 1.19 is the
cleaner fix where the distribution allows it.

When `xdg-desktop-portal` cannot be reached, `doctor` falls back to its own model of
the selection rules and says so.

The most common cause is that another backend wins the `FileChooser` interface. In
that case `doctor` names the config file responsible:

```
  [fail] selected FileChooser backend: gnome
         chosen by /home/user/.config/xdg-desktop-portal/niri-portals.conf
         this is why you get the 'gnome' file picker instead of a terminal one
         -> in /home/user/.config/xdg-desktop-portal/niri-portals.conf, set org.freedesktop.impl.portal.FileChooser=termfilepickers
```

Note that a desktop-specific file such as `niri-portals.conf` takes precedence over
`portals.conf` in the same directory, so adding your preference to the latter has no
effect when the former exists.

To also run the configured open-file script and open a real picker:

```sh
xdg-desktop-portal-termfilepickers doctor --run-scripts
```

Two things `doctor` can only point at, since they live outside the portal:

- GTK applications need `GTK_USE_PORTAL=1` (GTK 3) and `GDK_DEBUG=portals` (GTK 4)
  in the session environment.
- Firefox-based browsers need `widget.use-xdg-desktop-portal.file-picker` set to `1`
  in `about:config`.

Changing the environment requires logging out and back in. Changing only this
service's config needs just a restart:
`systemctl --user restart xdg-desktop-portal-termfilepickers`.

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
