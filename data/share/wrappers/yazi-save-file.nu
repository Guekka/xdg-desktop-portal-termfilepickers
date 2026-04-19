#!/usr/bin/env nu

def main [json: string] {
  let parsed_json = ($json | from json)
  let out_file = ($parsed_json | get "out_file")
  let termcmd = ($parsed_json | get "termcmd")
  let path = ($parsed_json | get "recommended_path")

  let yazi_args = ["--chooser-file" $out_file $path]

  run-external ...$termcmd "yazi" ...$yazi_args
}
