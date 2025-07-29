#!/usr/bin/env nu

def main [json: string] {
  let json = $json | from json
  let out_file = $json | get "out_file"
  let termcmd = $json | get "termcmd"
  let directory = $json | get "directory" | into bool

  let yazi_args = ["--chooser-file" $out_file]

  if $directory {
    $yazi_args | append ["--cwd-file" $out_file]
  }

  if $termcmd ends-with "ghostty" {
    let yazi_command = (["yazi"] | append $yazi_args | str join " ")
    run-external $termcmd "-e" $yazi_command
    return
  }

  run-external $termcmd "yazi" ...$yazi_args
}
