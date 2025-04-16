#!/usr/bin/env zsh

set -ex

disable -r time

MODE=release # debug or release
DIR=$1
BIN=$DIR/target/$MODE/examples/circ
ZK_BIN=$DIR/target/$MODE/examples/zk

modulus=28948022309329048855892746252171976963363056481941647379679742748393362948097

function spartan_r1cs {
    $BIN --field-custom-modulus $modulus $DIR/alpaca/relations/function_f.zok r1cs --action spartan-setup --prover-key IVC_P --verifier-key IVC_V
    $BIN --field-custom-modulus $modulus $DIR/alpaca/relations/relation_post.zok r1cs --action spartan-setup
}

spartan_r1cs