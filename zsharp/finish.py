import sys

pin_file_template_path = "./circ-mastadon/zsharp/relation_r_template.zok.pin"
vin_file_template_path = "./circ-mastadon/zsharp/relation_r_template.zok.vin"

def replace_file_contents(source_path, dest_path):
    with open(source_path, 'r') as source_file:
        content = source_file.read()

    with open(dest_path, 'w') as dest_file:
        dest_file.write(content)


if __name__ == "__main__":
    args = sys.argv[1:]
    pin_vin_flag = args[0]
    file_path = args[1]

    if pin_vin_flag == 0 or pin_vin_flag == '0':
        replace_file_contents(pin_file_template_path, file_path)
    elif pin_vin_flag == 1 or pin_vin_flag == '1':
        replace_file_contents(vin_file_template_path, file_path)
    else:
        raise Exception("Only 0 and 1 are allowed for args[1]")

    print(file_path + " has been rolled back to template file")

