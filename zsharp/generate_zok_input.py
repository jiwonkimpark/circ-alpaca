import sys

F_p = 28948022309329048855892746252171976963363056481941560715954676764349967630337
F_q = 28948022309329048855892746252171976963363056481941647379679742748393362948097

def hex_string_from(byte_array_str):
    array = byte_array_str.replace(' ', '').split('[')[1].split(']')[0].split(',')
    for i in range(0, len(array)):
        if len(array[i]) == 1:
            array[i] = '0' + array[i]
    final_string = ''
    for i in range(len(array)-1, -1, -1):
        final_string += array[i]
    return '0x' + final_string


def hex_to_int(hex_str):
    res = int(hex_str, 16)
    return res


def int_to_field(int_value, modulus):
    res = int_value % modulus
    return res


def field_from(hex_str):
    int_res = hex_to_int(hex_str)
    field_res = int_to_field(int_res, F_q)  # change F_q when needed
    return field_res


def zok_input_arguments_from(hex_str):
    field = field_from(hex_str)
    arg = "#f" + str(field)
    return arg


def zok_input_fields(args):
    input_list = []
    for arg in args:
        arg_hex = hex_string_from(arg)
        input_list.append(zok_input_arguments_from(arg_hex))
    return input_list


if __name__ == "__main__":
    args = sys.argv[1:]
    in_path = args[0]
    input_fields = zok_input_fields(args[1:])

    with open(in_path, 'r') as file:
        content = file.read()

    updated = content
    for i in range(0, len(input_fields)):
        if i < 10:
            updated = updated.replace('$0' + str(i), input_fields[i])
        else:
            updated = updated.replace('$' + str(i), input_fields[i])

    with open(in_path, 'w') as file:
        file.write(updated)

    print(in_path + " file has successfully been created")
