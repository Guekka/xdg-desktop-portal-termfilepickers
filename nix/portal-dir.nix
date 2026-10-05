# Builds a single directory containing every portal backend and the generated
# portals.conf, for use with XDG_DESKTOP_PORTAL_DIR.
#
# That variable is understood by unpatched xdg-desktop-portal builds, unlike the
# NIX_XDG_DESKTOP_PORTAL_DIR one that nixpkgs adds. It replaces the entire lookup,
# for backends and for portals.conf alike, so both have to live here together.
{
  pkgs,
  lib,
  portals, # packages providing share/xdg-desktop-portal/portals/*.portal
  config, # attrset of desktop -> { interface = [backends]; }
}: let
  # portals.conf wants semicolon separated lists; xdg.portal.config accepts either
  # a list or an already joined string, so normalise here.
  toConf = settings:
    lib.generators.toINI {} {
      preferred =
        lib.mapAttrs
        (_: value: lib.concatStringsSep ";" (lib.toList value))
        settings;
    };

  confFiles = lib.mapAttrs' (desktop: settings:
    lib.nameValuePair
    "${lib.optionalString (desktop != "common") "${desktop}-"}portals.conf"
    (toConf settings))
  (lib.filterAttrs (_: settings: settings != {}) config);
in
  pkgs.runCommand "xdg-desktop-portal-dir" {
    passthru = {inherit portals;};
  } ''
    mkdir -p $out

    for portal in ${lib.escapeShellArgs portals}; do
      dir="$portal/share/xdg-desktop-portal/portals"
      if [ -d "$dir" ]; then
        # first one wins, matching how xdg-desktop-portal itself dedupes by name
        for file in "$dir"/*.portal; do
          [ -e "$file" ] || continue
          name=$(basename "$file")
          if [ ! -e "$out/$name" ]; then
            ln -s "$file" "$out/$name"
          fi
        done
      fi
    done

    ${lib.concatStringsSep "\n" (lib.mapAttrsToList (name: text: ''
        cat > "$out/${name}" <<'PORTALS_CONF'
        ${text}
        PORTALS_CONF
      '')
      confFiles)}
  ''
