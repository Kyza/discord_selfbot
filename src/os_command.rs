use std::{
	io::{BufRead, BufReader},
	process::{self, Output},
	thread,
};

use anyhow::Result;

// A function that runs a command on the OS.
// It automatically prints the output and returns the exit code.
pub fn run_os_command(
	tag: impl ToString,
	mut command: process::Command,
) -> Result<Output> {
	let tag = tag.to_string();
	command.stdout(process::Stdio::piped());
	command.stderr(process::Stdio::piped());
	let mut command_output = command.spawn()?;
	thread::spawn(move || {
		let reader = BufReader::new(command_output.stdout.take().unwrap());
		for line in reader.lines() {
			let line = line.unwrap();
			println!("[{}] {}", tag, line);
		}
	});

	return Ok(command.output()?);
}
