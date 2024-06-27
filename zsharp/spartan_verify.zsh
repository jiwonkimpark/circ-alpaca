#!/usr/bin/env zsh

set -ex

disable -r time

MODE=release # debug or release
BIN=./circ-zsharp/target/$MODE/examples/circ
ZK_BIN=./circ-zsharp/target/$MODE/examples/zk

modulus=28948022309329048855892746252171976963363056481941647379679742748393362948097

function spartan_test_verify {
      ex_name=$1
      $BIN --field-custom-modulus $modulus ./circ-zsharp/zsharp/$ex_name.zok r1cs --action spartan-setup
      $ZK_BIN --field-custom-modulus $modulus --vin ./circ-zsharp/zsharp/$ex_name.zok.vin --action spartan-verify
      rm -rf P V pi
}

spartan_test_verify relation_r_tmp
