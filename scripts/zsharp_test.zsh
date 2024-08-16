#!/usr/bin/env zsh

set -ex

disable -r time

MODE=release
BIN=../target/$MODE/examples/zxi

modulus=28948022309329048855892746252171976963363056481941560715954676764349967630337

function simple_test {
  file_path=$1
  $BIN --field-custom-modulus $modulus "$file_path"
}

simple_test ../zsharp/simple_test.zok