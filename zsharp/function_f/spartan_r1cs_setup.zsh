#!/usr/bin/env zsh

set -ex

disable -r time

#cargo build --release --features r1cs,zok,spartan --example circ
#cargo build --release --features r1cs,zok,spartan --example zk

MODE=release # debug or release
#DIR=/Users/jiwonkim/research/tmp/Mastadon/circ-mastadon
DIR=/home/jiwonkp/mastodon/Mastadon/circ-mastadon
BIN=$DIR/target/$MODE/examples/circ
ZK_BIN=$DIR/target/$MODE/examples/zk

modulus=28948022309329048855892746252171976963363056481941647379679742748393362948097

function spartan_r1cs {
    ex_name=$1
    $BIN --field-custom-modulus $modulus $DIR/zsharp/function_f/$ex_name.zok r1cs --action spartan-setup --prover-key IVC_P --verifier-key IVC_V
}


spartan_r1cs function_f