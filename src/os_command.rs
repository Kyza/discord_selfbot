use std::{
	io::{BufRead, BufReader},
	process::{self, Output},
	thread,
};

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
pub fn run_os_command(
	tag: impl ToString,
	mut command: process::Command,
) -> Result<Output> {
	let tag = tag.to_string();
	println!("[{}] {}", tag, command_to_string(&command));

	command.stdout(process::Stdio::piped());
	command.stderr(process::Stdio::piped());

	let mut child = command.spawn()?;

	let stdout = child.stdout.take().expect("Failed to capture stdout");
	let stderr = child.stderr.take().expect("Failed to capture stderr");

	let stdout_tag = format!("[{}]", tag);
	let stderr_tag = format!("[{}]", tag);
	let stdout_thread = thread::spawn(move || {
		let stdout_reader = BufReader::new(stdout);
		for line in stdout_reader.lines() {
			if let Ok(line) = line {
				println!("{} {}", stdout_tag, line);
			}
		}
	});

	let stderr_thread = thread::spawn(move || {
		let stderr_reader = BufReader::new(stderr);
		for line in stderr_reader.lines() {
			if let Ok(line) = line {
				eprintln!("{} {}", stderr_tag, line);
			}
		}
	});

	// Wait for the command to complete and for both threads to finish
	child.wait()?;
	stdout_thread.join().expect("Stdout thread panicked");
	stderr_thread.join().expect("Stderr thread panicked");

	Ok(command.output()?)
}
