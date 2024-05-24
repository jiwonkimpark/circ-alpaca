def hex_to_int(hex_str):
	int_value = int(hex_str, 16)
	return int_value


def mds_matrix_int(matrix):
	row = len(matrix)
	col = len(matrix[0])
	result = [[0] * col for i in range(row)]

	for i in range(row):
		for j in range(col):
			result[i][j] = hex_to_int(matrix[i][j])
	
	return result


def mds_matrix_hex(file_name):
	file = open(file_name, 'r')
	return file.readlines([-1]) # mds matrix is in the last line


def write_to_file(matrices, file_name):
	file = open(file_name, 'w')
	file.write(matrices)


if __name__=="__main__":
	for i in range(2, 10):
		input_file = "poseidon_params_n255_t{}_alpha5_M128.txt".format(i)
		matrix_hex = mds_matrix_hex(input_file)
		matrix_int = mds_matrix_int(matrix_hex)

		output_file = "t{}_mds_matrix.txt".format(i)


