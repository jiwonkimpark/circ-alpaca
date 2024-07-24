use std::process::{Command, Stdio};
use std::time::Instant;

const CIRC_SPARTAN_R1CS_SCRIPT_PATH: &str = "/Users/jiwonkim/research/tmp/Mastadon/circ-mastadon/zsharp/function_f/spartan_r1cs_full_f.zsh";

fn run_shell_script(script_path: &str, args: Option<Vec<String>>) {
    let mut sh_args: Vec<String> = vec![script_path.parse().unwrap()];
    if args.is_some() {
        sh_args.extend(args.unwrap());
    }

    let mut child = Command::new("zsh")
        .args(sh_args)
        .spawn()
        .expect("Failed to execute script");

    // let mut stdout = match child.stdout.take() {
    //     Some(stdout) => stdout,
    //     None => panic!("Failed to capture stdout"),
    // };
    //
    // let mut output = String::new();
    // match stdout.read_to_string(&mut output) {
    //     Ok(_) => println!("Script output:\n{}", output),
    //     Err(e) => panic!("Failed to read stdout: {}", e),
    // }
    // output
}

fn main() {
    run_shell_script(CIRC_SPARTAN_R1CS_SCRIPT_PATH, None);
}