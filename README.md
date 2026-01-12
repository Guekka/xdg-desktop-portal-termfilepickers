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

### Terminal Command Configuration

The `terminal_command` must be compatible with how the wrapper scripts call it. The scripts will append the file manager command (e.g., `yazi --chooser-file /tmp/xyz`) to your terminal command.

**Working examples:**
```nix
# Kitty (recommended)
terminal_command = [(lib.getExe pkgs.kitty)];

# Alacritty
terminal_command = [(lib.getExe pkgs.alacritty) "-e"];

# Wezterm
terminal_command = [(lib.getExe pkgs.wezterm) "-e"];

# Foot
terminal_command = [(lib.getExe pkgs.foot) "-e"];

# Ghostty - Note: Ghostty may have different flag syntax
# Check `ghostty --help` for the correct execute flag
terminal_command = [(lib.getExe pkgs.ghostty) "-e"];
```

**Important:** Test your terminal command works by running:
```bash
# Replace with your terminal_command
kitty yazi --chooser-file /tmp/test.txt
# Or with -e flag:
alacritty -e yazi --chooser-file /tmp/test.txt
```

If the terminal doesn't open or yazi doesn't start, your `terminal_command` configuration needs adjustment.

## Configuration

### Desktop Environment Detection

The `desktopEnvironments` setting determines which portal configuration files are created. XDP uses `$XDG_CURRENT_DESKTOP` to decide which configuration to load.

**Important:** The desktop environment name must match what XDP detects (case-insensitive in config, but must match the detected value).

Check your current desktop:
```bash
echo $XDG_CURRENT_DESKTOP
```

Common configurations:
```nix
# For Hyprland
desktopEnvironments = ["hyprland"];

# For Sway
desktopEnvironments = ["sway"];

# For multiple or fallback
desktopEnvironments = ["hyprland" "common"];
```

If your desktop environment isn't detected correctly, you may need to add multiple entries or use "common" as a fallback.

### Service Configuration

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

2. **TEST YOUR TERMINAL COMMAND MANUALLY (Critical!):**
   This is the most important debugging step. Test if your terminal command works with yazi:
   ```bash
   # Test your exact terminal command
   # Replace with your configured terminal_command
   kitty yazi --chooser-file /tmp/test.txt
   # OR with -e flag if configured:
   alacritty -e yazi --chooser-file /tmp/test.txt
   ```
   
   - The terminal should open with yazi running
   - Select a file and press Enter
   - Check if /tmp/test.txt was created with the file path
   
   **If this doesn't work, your `terminal_command` configuration is incorrect!**
   
   Common issues:
   - Wrong execute flag (some terminals use `-e`, some use `--command`, some need no flag)
   - Ghostty users: Check `ghostty --help` for correct syntax
   - Missing yazi in PATH

3. **Check termfilepickers service logs for errors:**
   ```bash
   journalctl --user -u xdg-desktop-portal-termfilepickers.service -n 50
   ```
   Look for error messages like "Runner failed" or "Script did not produce a valid output file".
   These indicate the terminal command or file manager failed to execute properly.

4. **Verify DBus service file is installed:**
   ```bash
   ls -l ~/.local/share/dbus-1/services/org.freedesktop.impl.portal.desktop.termfilepickers.service
   ```
   This file must exist for DBus to activate the service. If it's missing, rebuild your configuration.

5. Check that termfilepickers service is running:
   ```bash
   systemctl --user status xdg-desktop-portal-termfilepickers.service
   ```

6. Check the xdg-desktop-portal configuration:
   ```bash
   cat ~/.config/xdg-desktop-portal/<your-desktop>-portals.conf
   ```
   It should contain:
   ```ini
   [preferred]
   org.freedesktop.impl.portal.FileChooser=termfilepickers
   ```

7. **Check XDP logs to verify termfilepickers is actually being called:**
   ```bash
   # Stop the portal service
   systemctl --user stop xdg-desktop-portal.service
   
   # Run it in verbose mode
   /nix/store/*/libexec/xdg-desktop-portal -vr
   
   # In another terminal, test the file picker
   GTK_USE_PORTAL=1 zenity --file-selection
   ```
   
   Look for these critical lines in order:
   ```
   XDP: Using termfilepickers.portal for org.freedesktop.impl.portal.FileChooser (config)
   XDP: Handling OpenFile
   ```
   
   **If you see "Using termfilepickers.portal" but NOT "Handling OpenFile":**
   - XDP recognized termfilepickers but isn't calling it
   - Check if another portal is intercepting the call
   - Verify your desktop environment is detected correctly
   
   **If you see both lines:**
   - termfilepickers IS being called
   - Check termfilepickers logs: `journalctl --user -u xdg-desktop-portal-termfilepickers.service -f`
   - Look for "Runner failed" or other error messages

8. **Verify desktop environment detection:**
   ```bash
   echo $XDG_CURRENT_DESKTOP
   ```
   This should match one of your configured `desktopEnvironments`. If it doesn't match, XDP won't use your portal configuration!
   
   Common values: `Hyprland`, `sway`, `GNOME`, `KDE`. Note: case-sensitive!
   
   If the value doesn't match, add it to your `desktopEnvironments` list (lowercase).

9. After configuration changes, make sure to restart both services in order:
   ```bash
   systemctl --user restart xdg-desktop-portal-termfilepickers.service
   systemctl --user restart xdg-desktop-portal.service
   ```

10. **If the service still doesn't work after updating:**
    - Verify the DBus service file path: `cat ~/.local/share/dbus-1/services/org.freedesktop.impl.portal.desktop.termfilepickers.service`
    - Check that it points to the correct executable
    - Reload DBus: `systemctl --user daemon-reload`
    - Restart your user session or reboot to ensure all DBus changes are applied

### Common Root Causes

If you've gone through all troubleshooting steps and it still doesn't work, here are the most common root causes:

1. **Desktop environment mismatch**: `$XDG_CURRENT_DESKTOP` doesn't match your `desktopEnvironments` configuration
2. **Application doesn't use portals**: Some applications bypass XDP entirely (check application-specific documentation)
3. **Portal priority conflict**: Another portal is handling FileChooser with higher priority
4. **XDP not actually calling termfilepickers**: Check for "Handling OpenFile" in XDP verbose logs
5. **Service errors**: Check termfilepickers logs for "Runner failed" or similar errors

### Notes

- Some applications don't use the portal and will use their native file picker instead
- See [tips and compatibility](https://github.com/hunkyburrito/xdg-desktop-portal-termfilechooser#testing-and-tips) from the related project

_This documentation is work in progress_
