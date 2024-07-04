import sys

pin_file_template_path = "./circ-mastadon/zsharp/function_f_template.zok.pin"

def replace_file_contents(source_path, dest_path):
    with open(source_path, 'r') as source_file:
        content = source_file.read()

    with open(dest_path, 'w') as dest_file:
        dest_file.write(content)


if __name__ == "__main__":
    args = sys.argv[1:]
    file_path = args[0]

    replace_file_contents(pin_file_template_path, file_path)

    print(file_path + " has been rolled back to template file")

