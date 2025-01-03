use thirtyfour::prelude::*;
use url::Url;

use crate::types::Context;
use anyhow::Result;
use inline_format::println;
use poise::{
	serenity_prelude::{
		CreateActionRow, CreateAllowedMentions, CreateAttachment,
		CreateButton,
	},
	CreateReply,
};

// TODO: Create context menu version.

/// Screenshots a website.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn screenshot(
	ctx: Context<'_>,
	#[description = "The URL to screenshot."] url: Url,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let ephemeral = ephemeral.unwrap_or(false);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	reply = reply.components(vec![CreateActionRow::Buttons(vec![
		CreateButton::new_link(url.to_string()).label("View Online"),
	])]);
	reply = reply.attachment(CreateAttachment::bytes(
		screenshot_url(&url).await?,
		"screenshot.png",
	));

	ctx.send(reply).await?;

	Ok(())
}

pub async fn screenshot_url(url: &Url) -> Result<Vec<u8>> {
	println!("Starting Firefox.");

	let mut caps = DesiredCapabilities::firefox();
	// If in debug mode, run non-headless.
	// Look at him he has Smitty Werbenjägermanjensen's hat.
	if !cfg!(debug_assertions) {
		caps.set_headless()?;
	}
	let driver = WebDriver::new("http://localhost:4444", caps).await?;

	driver.goto(url.to_string()).await?;

	// Wait for the page to load.
	let png = driver.screenshot_as_png().await?;

	Ok(png)
}

// async fn start_geckodriver() -> Result<Child> {
// 	let mut geckodriver = process::Command::new("geckodriver")
// 		.arg("--port=4444")
// 		.stdout(process::Stdio::piped())
// 		.stderr(process::Stdio::piped())
// 		.spawn()
// 		.expect("Failed to start geckodriver");

// 	// Get handles to stdout and stderr
// 	let stdout = geckodriver.stdout.take().unwrap();
// 	let stderr = geckodriver.stderr.take().unwrap();

// 	// Create readers
// 	let stdout_reader = std::io::BufReader::new(stdout);
// 	let stderr_reader = std::io::BufReader::new(stderr);

// 	// Wait for any output (either from stdout or stderr)
// 	use std::io::BufRead;
// 	let (tx, rx) = std::sync::mpsc::channel();

// 	// Monitor stdout
// 	let tx_stdout = tx.clone();
// 	std::thread::spawn(move || {
// 		if let Some(line) = stdout_reader.lines().next() {
// 			if let Ok(line) = line {
// 				tx_stdout.send(line).ok();
// 			}
// 		}
// 	});

// 	// Monitor stderr
// 	std::thread::spawn(move || {
// 		if let Some(line) = stderr_reader.lines().next() {
// 			if let Ok(line) = line {
// 				tx.send(line).ok();
// 			}
// 		}
// 	});

// 	// Wait for the first line of output
// 	let a = rx.recv().expect("Failed to get geckodriver output");
// 	println!("geckodriver output: {}", a);

// 	// Sleep for a second to ensure it's ready.
// 	tokio::time::sleep(Duration::from_millis(1000)).await;

// 	Ok(geckodriver)
// }
