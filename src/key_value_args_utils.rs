// These functions are copied from Poise's source code because they're not public and I need them. :)

// MIT License

// Copyright (c) 2021 kangalioo

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use poise::{KeyValueArgs, TooFewArguments};

/// Pop a whitespace-separated word from the front of the arguments. Supports quotes and quote
/// escaping.
///
/// Leading whitespace will be trimmed; trailing whitespace is not consumed.
pub fn pop_string(args: &str) -> Result<(&str, String), TooFewArguments> {
	// TODO: consider changing the behavior to parse quotes literally if they're in the middle
	// of the string:
	// - `"hello world"` => `hello world`
	// - `"hello "world"` => `"hello "world`
	// - `"hello" world"` => `hello`

	let args = args.trim_start();
	if args.is_empty() {
		return Err(TooFewArguments::default());
	}

	let mut output = String::new();
	let mut inside_string = false;
	let mut escaping = false;

	let mut chars = args.chars();
	// .clone().next() is poor man's .peek(), but we can't do peekable because then we can't
	// call as_str on the Chars iterator
	while let Some(c) = chars.clone().next() {
		if escaping {
			output.push(c);
			escaping = false;
		} else if !inside_string && c.is_whitespace() {
			break;
		} else if c == '"' {
			inside_string = !inside_string;
		} else if c == '\\' {
			escaping = true;
		} else {
			output.push(c);
		}

		chars.next();
	}

	Ok((chars.as_str(), output))
}

/// Reads a single key value pair ("key=value") from the front of the arguments
pub fn pop_single_key_value_pair(
	args: &str,
) -> Option<(&str, (String, String))> {
	// TODO: share quote parsing machinery with PopArgumentAsync impl for String

	if args.is_empty() {
		return None;
	}

	let mut key = String::new();
	let mut inside_string = false;
	let mut escaping = false;

	let mut chars = args.trim_start().chars();
	loop {
		let c = chars.next()?;
		if escaping {
			key.push(c);
			escaping = false;
		} else if !inside_string && c.is_whitespace() {
			return None;
		} else if c == '"' {
			inside_string = !inside_string;
		} else if c == '\\' {
			escaping = true;
		} else if !inside_string && c == '=' {
			break;
		} else if !inside_string && c.is_ascii_punctuation() {
			// If not enclosed in quotes, keys mustn't contain special characters.
			// Otherwise this command invocation: "?eval `0..=5`" is parsed as key-value args
			// with key "`0.." and value "5`". (This was a long-standing issue in rustbot)
			return None;
		} else {
			key.push(c);
		}
	}

	let args = chars.as_str();
	// `args` used to contain "key=value ...", now it contains "value ...", so pop the value off
	let (args, value) = pop_string(args).unwrap_or((args, String::new()));

	Some((args, (key, value)))
}

/// Reads as many key-value args as possible from the front of the string and produces a
/// [`KeyValueArgs`] out of those
pub fn pop_from(mut args: &str) -> (&str, KeyValueArgs) {
	let mut pairs = std::collections::HashMap::new();

	while let Some((remaining_args, (key, value))) =
		pop_single_key_value_pair(args)
	{
		args = remaining_args;
		pairs.insert(key, value);
	}

	(args, KeyValueArgs(pairs))
}
