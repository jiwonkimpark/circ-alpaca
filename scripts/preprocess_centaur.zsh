#!/usr/bin/env zsh

set -ex

disable -r time

MODE=release # debug or release
DIR=$1
BIN=$DIR/target/$MODE/examples/circ
ZK_BIN=$DIR/target/$MODE/examples/zk
#CALLBACK_CAPACITY=$2
#BB_CAPACITY=$3

modulus=28948022309329048855892746252171976963363056481941647379679742748393362948097

function preprocess_r1cs {
    RUST_BACKTRACE=full $BIN --field-custom-modulus $modulus $DIR/examples/ZoKrates/centaur/scan_new.zok r1cs --action spartan-setup --prover-key SCAN_NEW_P --verifier-key SCAN_NEW_V
    RUST_BACKTRACE=full $BIN --field-custom-modulus $modulus $DIR/examples/ZoKrates/centaur/test.zok r1cs --action spartan-setup --prover-key SCAN_OWN_P --verifier-key SCAN_OWN_V
#    $BIN --field-custom-modulus $modulus $DIR/examples/ZoKrates/centaur/well_formed_$BB_CAPACITY.zok r1cs --action spartan-setup --prover-key WELL_FORMED_P --verifier-key WELL_FORMED_V
}

preprocess_r1cs