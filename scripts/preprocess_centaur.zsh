#!/usr/bin/env zsh

set -ex

disable -r time

MODE=release # debug or release
DIR=/Users/jiwonkim/research/circ-alpaca
BIN=$DIR/target/$MODE/examples/circ
ZK_BIN=$DIR/target/$MODE/examples/zk

modulus=28948022309329048855892746252171976963363056481941647379679742748393362948097

function preprocess_r1cs {
    $BIN --field-custom-modulus $modulus $DIR/examples/ZoKrates/centaur/scan_new.zok r1cs --action spartan-setup --prover-key SCAN_NEW_P --verifier-key SCAN_NEW_V
    $BIN --field-custom-modulus $modulus $DIR/examples/ZoKrates/centaur/scan_own.zok r1cs --action spartan-setup --prover-key SCAN_OWN_P --verifier-key SCAN_OWN_V
    $BIN --field-custom-modulus $modulus $DIR/examples/ZoKrates/centaur/well_formed.zok r1cs --action spartan-setup --prover-key WELL_FORMED_P --verifier-key WELL_FORMED_V
}

preprocess_r1cs