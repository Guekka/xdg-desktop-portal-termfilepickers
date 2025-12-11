# XDG Desktop Portal Termfilepickers

This is a desktop portal for file picking and saving, which is used by Flatpak applications.

I have been daily-driving it for the past few months to replace [xdg-desktop-portal-termfilechooser](https://github.com/exquo/xdg-desktop-portal-termfilechooser/)

## Requirements

The default wrapper scripts require:
- **[Nushell](https://www.nushell.sh/)** - Automatically included as a runtime dependency in the Nix package
- **File manager** (default is `yazi`) - Must be installed separately:
  ```nix
  environment.systemPackages = [ pkgs.yazi ];  # NixOS
  # or
  home.packages = [ pkgs.yazi ];  # Home Manager
  ```

**Note:** When using the Nix package, nushell is automatically included and doesn't need to be in your PATH. For non-Nix installations, you'll need to install nushell separately.

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

## Configuration

Ensure you have `xdg.portal.enable = true` in your configuration. The module will automatically:
- Install the portal backend
- Configure the desktop portal to use termfilepickers for FileChooser interface
- Set up the systemd service for DBus activation
- Link the DBus service file to enable proper service discovery

After updating your configuration, you must restart both services in this order:
```bash
systemctl --user restart xdg-desktop-portal-termfilepickers.service
systemctl --user restart xdg-desktop-portal.service
```

**Important:** Make sure to rebuild your Home Manager or NixOS configuration to get the latest changes, including the DBus service file installation.

## Testing

Test the file picker with:
```bash
GTK_USE_PORTAL=1 zenity --file-selection
```

You can also test with additional options:
- `--multiple` - Select multiple files
- `--directory` - Select a directory
- `--save` - Save file dialog

## Troubleshooting

### File picker doesn't appear or uses the wrong backend

1. **Verify required dependencies are installed:**
   ```bash
   which yazi  # Should return the path to your file manager
   ```
   If yazi (or your chosen file manager) is missing, install it:
   ```nix
   # NixOS
   environment.systemPackages = [ pkgs.yazi ];
   # Home Manager
   home.packages = [ pkgs.yazi ];
   ```
   **This is the most common issue!** The file manager must be installed separately.
   
   Note: Nushell is automatically included in the Nix package, so you don't need to install it separately.

2. **Verify DBus service file is installed:**
   ```bash
   ls -l ~/.local/share/dbus-1/services/org.freedesktop.impl.portal.desktop.termfilepickers.service
   ```
   This file must exist for DBus to activate the service. If it's missing, rebuild your configuration.

3. Check that termfilepickers service is running:
   ```bash
   systemctl --user status xdg-desktop-portal-termfilepickers.service
   ```

4. Check the xdg-desktop-portal configuration:
   ```bash
   cat ~/.config/xdg-desktop-portal/<your-desktop>-portals.conf
   ```
   It should contain:
   ```ini
   [preferred]
   org.freedesktop.impl.portal.FileChooser=termfilepickers
   ```

5. Check xdg-desktop-portal logs to see which backend is being used:
   ```bash
   # Stop the portal service
   systemctl --user stop xdg-desktop-portal.service
   
   # Run it in verbose mode
   /nix/store/*/libexec/xdg-desktop-portal -vr
   
   # In another terminal, test the file picker
   GTK_USE_PORTAL=1 zenity --file-selection
   ```
   
   Look for lines like:
   ```
   XDP: Using termfilepickers.portal for org.freedesktop.impl.portal.FileChooser
   ```

6. After configuration changes, make sure to restart both services in order:
   ```bash
   systemctl --user restart xdg-desktop-portal-termfilepickers.service
   systemctl --user restart xdg-desktop-portal.service
   ```

7. **Check termfilepickers service logs for errors:**
   ```bash
   journalctl --user -u xdg-desktop-portal-termfilepickers.service -f
   ```
   Look for error messages when you try to open a file. Common errors include:
   - Terminal command failures (wrong terminal arguments)
   - File manager not found (yazi not installed)

8. **If the service still doesn't work after updating:**
   - Verify the DBus service file path: `cat ~/.local/share/dbus-1/services/org.freedesktop.impl.portal.desktop.termfilepickers.service`
   - Check that it points to the correct executable
   - Reload DBus: `systemctl --user daemon-reload`
   - Restart your user session or reboot to ensure all DBus changes are applied

### Notes

- Some applications don't use the portal and will use their native file picker instead
- See [tips and compatibility](https://github.com/hunkyburrito/xdg-desktop-portal-termfilechooser#testing-and-tips) from the related project

_This documentation is work in progress_
