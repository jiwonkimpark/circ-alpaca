import sys

pin_file_template_path = "./circ-mastadon/zsharp/function_f/function_f_template.zok.pin"

com_k_template_path = "./circ-mastadon/zsharp/function_f/function_f_com_k_template.zok.pin"
sign_start_template_path = "./circ-mastadon/zsharp/function_f/function_f_sign_start_template.zok.pin"
hash_template_path = "./circ-mastadon/zsharp/function_f/function_f_hash_template.zok.pin"
banned_template_path = "./circ-mastadon/zsharp/function_f/function_f_banned_template.zok.pin"


def replace_file_contents(source_path, dest_path):
    with open(source_path, 'r') as source_file:
        content = source_file.read()

    with open(dest_path, 'w') as dest_file:
        dest_file.write(content)


if __name__ == "__main__":
    args = sys.argv[1:]
    enforcement_flag = args[0]
    file_path = args[1]

    if enforcement_flag == 0 or enforcement_flag == '0':
        replace_file_contents(com_k_template_path, file_path)
    elif enforcement_flag == 1 or enforcement_flag == '1':
        replace_file_contents(sign_start_template_path, file_path)
    elif enforcement_flag == 2 or enforcement_flag == '2':
        replace_file_contents(com_k_template_path, file_path)
    elif enforcement_flag == 3 or enforcement_flag == '3':
        replace_file_contents(hash_template_path, file_path)
    elif enforcement_flag == 4 or enforcement_flag == '4':
        replace_file_contents(banned_template_path, file_path)
    else:
        raise Exception("Only numbers from 0 to 4 are allowed for args[0]")

    print(file_path + " has been rolled back to template file")

