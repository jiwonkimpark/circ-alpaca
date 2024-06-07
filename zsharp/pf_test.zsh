#!/usr/bin/env zsh

set -ex

disable -r time

cargo build --release --features r1cs,zok,spartan --example circ
cargo build --release --features r1cs,zok,spartan --example zk

MODE=release # debug or release
BIN=../target/$MODE/examples/circ
ZK_BIN=../target/$MODE/examples/zk

case "$OSTYPE" in
    darwin*)
        alias measure_time="gtime --format='%e seconds %M kB'"
    ;;
    linux*)
        alias measure_time="time --format='%e seconds %M kB'"
    ;;
esac

modulus=28948022309329048855892746252171976963363056481941647379679742748393362948097

function r1cs_test {
    zpath=$1
#    measure_time $BIN --field-custom-modulus $modulus $zpath r1cs --action count
    $BIN --field-custom-modulus $modulus $zpath r1cs --action count
}

function r1cs_test_count {
    zpath=$1
    threshold=$2
    o=$($BIN --field-custom-modulus $modulus $zpath r1cs --action count)
    n_constraints=$(echo $o | grep 'Final R1cs size:' | grep -Eo '\b[0-9]+\b')
    [[ $n_constraints -lt $threshold ]] || (echo "Got $n_constraints, expected < $threshold" && exit 1)
}

# Test prove workflow, given an example name
function spartan_test {
    ex_name=$1
    $BIN --field-custom-modulus $modulus ./$ex_name.zok r1cs --action spartan-setup
    $ZK_BIN --field-custom-modulus $modulus --pin ./$ex_name.zok.pin --vin ./$ex_name.zok.vin --action spartan
    rm -rf P V pi
}


# r1cs_test ./relation_r.zok
spartan_test relation_r_2

