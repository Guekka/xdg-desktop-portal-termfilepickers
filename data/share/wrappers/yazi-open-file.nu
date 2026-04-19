#!/usr/bin/env nu

def main [json: string] {
  let parsed_json = ($json | from json)
  let out_file = ($parsed_json | get "out_file")
  let termcmd = ($parsed_json | get "termcmd")
  let directory = ($parsed_json | get "directory" | into bool)

  let yazi_args = ["--chooser-file" $out_file]

  if $directory {
    $yazi_args | append ["--cwd-file" $out_file]
  }

  run-external ...$termcmd "yazi" ...$yazi_args
}
