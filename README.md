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

## Configuration

Ensure you have `xdg.portal.enable = true` in your configuration. The module will automatically:
- Install the portal backend
- Configure the desktop portal to use termfilepickers for FileChooser interface
- Set up the systemd service for DBus activation

After updating your configuration, restart the xdg-desktop-portal service:
```bash
systemctl --user restart xdg-desktop-portal.service
```

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

1. Check that termfilepickers service is running:
   ```bash
   systemctl --user status xdg-desktop-portal-termfilepickers.service
   ```

2. Check the xdg-desktop-portal configuration:
   ```bash
   cat ~/.config/xdg-desktop-portal/<your-desktop>-portals.conf
   ```
   It should contain:
   ```ini
   [preferred]
   org.freedesktop.impl.portal.FileChooser=termfilepickers
   ```

3. Check xdg-desktop-portal logs to see which backend is being used:
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

4. After configuration changes, make sure to restart both services:
   ```bash
   systemctl --user restart xdg-desktop-portal-termfilepickers.service
   systemctl --user restart xdg-desktop-portal.service
   ```

### Notes

- Some applications don't use the portal and will use their native file picker instead
- See [tips and compatibility](https://github.com/hunkyburrito/xdg-desktop-portal-termfilechooser#testing-and-tips) from the related project

_This documentation is work in progress_
