import os
import shutil
import sys
import tempfile

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


def pin_arg_from(hex_str):
    field = field_from(hex_str)
    arg = "#f" + str(field)
    return arg


def pin_list_from(args):
    pin_list = []
    for arg in args:
        arg_hex = hex_string_from(arg)
        pin_list.append(pin_arg_from(arg_hex))
    return pin_list


if __name__ == "__main__":
    args = sys.argv[1:]
    pin_list = pin_list_from(args)

    pin_path = "./circ-zsharp/zsharp/relation_r_tmp.zok.pin"

    # with tempfile.NamedTemporaryFile(delete=False) as temp_file:
    #     temp_file_path = temp_file.name
    #     shutil.copyfile(src=pin_path, dst=temp_file_path)

    with open(pin_path, 'r') as file:
        content = file.read()

    updated = content
    for i in range(0, len(pin_list)):
        updated = updated.replace('$' + str(i), str(pin_list[i]))

    with open(pin_path, 'w') as file:
        file.write(updated)

    # shutil.copyfile(src=temp_file_path, dst=pin_path)
    # os.remove(temp_file_path)

    print("pin file created")
