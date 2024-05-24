def hex_to_int(hex_str):
	int_value = int(hex_str, 16)
	return int_value


def int_constants(file_name):
	constants = []

	file = open(file_name, 'r')
	hex_consts = file.readlines()[0].split(',')

	for hex_const in hex_consts:
		hex_str = hex_const.split('\'')[1]
		int_const = hex_to_int(hex_str)
		constants.append(int_const)
	
	return constants


def write_to_file(consts, file_name):
	file = open(file_name, 'w')
	for const in consts:
		file.write(f"{const},\n")


if __name__=="__main__":
	for i in range(2, 10):
		input_file = "t{}_constants.txt".format(i)
		constants = int_constants(input_file)
		output_file = "t{}_constants_converted.txt".format(i)
		write_to_file(constants, output_file)
