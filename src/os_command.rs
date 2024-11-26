use std::process::{self, Output};

use anyhow::Result;

pub fn command_to_string(cmd: &process::Command) -> String {
	// Get the program name
	let program = cmd.get_program().to_str().unwrap_or("unknown");

	// Collect and convert arguments to strings
	let args = cmd
		.get_args()
		.map(|arg| arg.to_str().unwrap_or(""))
		.collect::<Vec<_>>()
		.join(" ");

	// Combine program and arguments
	if args.is_empty() {
		program.to_string()
	} else {
		format!("{} {}", program, args)
	}
}

// A function that runs a command on the OS.
// It automatically prints the output and returns the exit code.
pub fn run_os_command(mut command: process::Command) -> Result<Output> {
	println!("{}", command_to_string(&command));

	command
		// .stdout(Stdio::inherit())
		// .stderr(Stdio::inherit())
		.spawn()?;

	Ok(command.output()?)
}
