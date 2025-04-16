from field_to_bits import field_to_binary, binary_to_le_bits
from hex_to_field import hex_to_field

F_p = 28948022309329048855892746252171976963363056481941560715954676764349967630337
F_q = 28948022309329048855892746252171976963363056481941647379679742748393362948097


def le_bits(field):
    binary = field_to_binary(field)
    le_bits = binary_to_le_bits(binary)
    return le_bits


def bits_to_field_space(input_bits, modulus):
    mult = 1 % modulus
    val = 0 % modulus

    for bit in input_bits:
        if bit:
            val += mult
            val %= modulus

        mult = (mult + mult) % modulus

    return val


if __name__=="__main__":
    # hex_fq_field = '0x36adf9ffd9696ff4d0d4a69edb02fb4e736220d23b5c1cdb9bf94422a83a6d71'
    fq_field = 11083494930597483593429666963504709976368250795110563604438578267206049130243
    print("Before: {}", fq_field)
    # hex_fp_field = '0x36adf9ffd9696ff4d0d4a69edb02fb4e736220d23b5c1cdb9bf94422a83a6d71'
    # fp_field = hex_to_field(hex_fp_field, F_p)
    # print("Wanted: {}", fp_field)
    fq_field_le_bits = le_bits(fq_field)
    converted = bits_to_field_space(fq_field_le_bits, F_p)
    print("Converted: {}", converted)

    # base h (C2::Base): 0x36adf9ffd9696ff4d0d4a69edb02fb4e736220d23b5c1cdb9bf94422a83a6d71
    # scalar h (C2::Scalar): 0x36adf9ffd9696ff4d0d4a69edb02fb4e736220d23b5c1cdb9bf94422a83a6d71
