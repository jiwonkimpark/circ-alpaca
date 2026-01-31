#!/usr/bin/env zsh

set -ex

disable -r time

MODE=release # debug or release
DIR=$1
BIN=$DIR/target/$MODE/examples/circ
ZK_BIN=$DIR/target/$MODE/examples/zk
BB_CAPACITY=$2

modulus=28948022309329048855892746252171976963363056481941647379679742748393362948097

function preprocess_r1cs {
  $BIN --field-custom-modulus $modulus $DIR/examples/ZoKrates/centaur/well_formed_$BB_CAPACITY.zok r1cs --action spartan-setup --prover-key WELL_FORMED_P --verifier-key WELL_FORMED_V
}

preprocess_r1cs