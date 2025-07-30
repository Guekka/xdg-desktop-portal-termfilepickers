#!/usr/bin/env nu

def main [json: string] {
  let json = $json | from json
  let out_file = $json | get "out_file"
  let termcmd = $json | get "termcmd"
  let path = $json | get "recommended_path"

  let yazi_args = ["--chooser-file" $out_file $path]

  if $termcmd ends-with "ghostty" {
    run-external $termcmd "-e" "yazi" ...$yazi_args
    return
  }

  run-external $termcmd "yazi" ...$yazi_args
}
