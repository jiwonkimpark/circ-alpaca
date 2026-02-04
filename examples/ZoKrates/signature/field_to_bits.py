def field_to_binary(field):
    return bin(field)


def binary_to_le_bits(binary):
    bin_string = str(binary)
    print(bin_string)
    bit_length = len(bin_string) - 2  # binary is in 0bxxxx form
    print(bit_length)
    print(len(bin_string))

    le_bits = []
    for i in range(len(bin_string) - 1, 1, -1):
        if bin_string[i] == '1':
            le_bits.append(True)
        else:
            le_bits.append(False)

    le_bits_length = len(le_bits)
    for i in range(le_bits_length, 256):
        le_bits.append(False)

    return le_bits


def bit_array_for_zok(bit_array):
    res = ""
    res += "["
    for i in range(0, len(bit_array)):
        if bit_array[i]:
            res += "true, "
        else:
            res += "false, "

    res += "]"
    return res


if __name__=="__main__":
    field = 3719920270002846849733653064214732606247931240936760284211304969336821021866
    binary = field_to_binary(field)
    le_bits = binary_to_le_bits(binary)
    print(le_bits)
    le_bits_zok = bit_array_for_zok(le_bits)
    print(le_bits_zok)

