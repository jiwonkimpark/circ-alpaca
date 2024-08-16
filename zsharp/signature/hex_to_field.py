F_p = 28948022309329048855892746252171976963363056481941560715954676764349967630337
F_q = 28948022309329048855892746252171976963363056481941647379679742748393362948097


def hex_to_int(hex_str):
    res = int(hex_str, 16)
    return res


def int_to_field(int_value, modulus):
    res = int_value % modulus
    return res


def hex_to_field(hex_str, modulus):
    int_res = hex_to_int(hex_str)
    field_res = int_to_field(int_res, modulus)
    return field_res


if __name__=="__main__":
    hex_str = '0x2e837f148a7c5510a0c660f60c35977963192d8c9405e074cba72a20e40831e7'
    field_value = hex_to_field(hex_str, F_q)
    print(field_value)
