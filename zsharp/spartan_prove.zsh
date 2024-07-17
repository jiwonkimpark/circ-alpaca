#!/usr/bin/env zsh

#cargo build --release --features r1cs,zok,spartan --example circ
#cargo build --release --features r1cs,zok,spartan --example zk

set -ex

disable -r time

MODE=release # debug or release
BIN=./circ-mastadon/target/$MODE/examples/circ
ZK_BIN=./circ-mastadon/target/$MODE/examples/zk

modulus=28948022309329048855892746252171976963363056481941647379679742748393362948097

function spartan_test_prove {
      ex_name=$1
      $ZK_BIN --field-custom-modulus $modulus --pin ./circ-mastadon/zsharp/$ex_name.zok.pin --action spartan-prove
}

spartan_test_prove relation_r_tmp
