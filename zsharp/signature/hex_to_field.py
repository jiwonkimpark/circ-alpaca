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
    hex_str = '0x0839667774a319ed574d9c263ef4fb27edefbee1934b1953d35c222525867caa'
    field_value = hex_to_field(hex_str, F_p)
    print(field_value)
