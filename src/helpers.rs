use std::{
	fs::{self},
	path::{Path, PathBuf},
};

use anyhow::{anyhow, Error, Result};
use poise::serenity_prelude::{self as serenity, Attachment, EmbedThumbnail};
use reqwest::header;

use crate::types::{BotData, Context};

#[macro_export]
macro_rules! crunch {
	($($name:ident),* $(,)?) => {
		$(
			mod $name;
			#[allow(ambiguous_glob_reexports)]
			pub use $name::*;
		)*
	};
}
pub use crunch;

pub fn safe_delete(path: &PathBuf) -> Result<bool> {
	if fs::exists(path)? {
		fs::remove_file(path)?;
		Ok(true)
	} else {
		Ok(false)
	}
}

#[derive(Debug)]
pub enum AttachmentOrThumbnail {
	Attachment(Attachment),
	Embed(EmbedThumbnail),
}
impl AttachmentOrThumbnail {
	pub async fn download(
		&self,
		client: &reqwest::Client,
	) -> Result<Vec<u8>> {
		match self {
			AttachmentOrThumbnail::Attachment(a) => Ok(a.download().await?),
			AttachmentOrThumbnail::Embed(e) => {
				// Download the image from the proxy URL.
				let url = e.proxy_url.as_ref().ok_or_else(|| {
					anyhow!("Embed thumbnail has no proxy URL")
				})?;
				let request = client.get(url).send().await?;
				Ok(request.bytes().await?.to_vec())
			}
		}
	}

	pub fn filename(&self) -> String {
		match self {
			AttachmentOrThumbnail::Attachment(a) => a.filename.clone(),
			AttachmentOrThumbnail::Embed(e) => {
				// Parse the URL to get the filename.
				let url = &e.proxy_url;
				let url = if let Some(url) = url {
					url::Url::parse(&url).unwrap_or_else(|_| {
						url::Url::parse("https://example.com/thumbnail.png")
							.unwrap()
					})
				} else {
					return "thumbnail.png".to_string();
				};
				if let Some(path_segments) = url.path_segments() {
					let filename = path_segments
						.last()
						.unwrap_or_else(|| "thumbnail.png");
					filename.to_string()
				} else {
					"thumbnail.png".to_string()
				}
			}
		}
	}
}

// pub fn wait_for_file(path: &PathBuf) -> Result<()> {
// 	loop {
// 		match OpenOptions::new().read(true).write(true).open(path) {
// 			Ok(_) => return Ok(()),
// 			Err(e) if e.kind() == ErrorKind::PermissionDenied => {
// 				// File is still in use; wait and retry.
// 				thread::sleep(Duration::from_millis(100));
// 			}
// 			// Handle other errors.
// 			Err(e) => return Err(anyhow!(e)),
// 		}
// 	}
// }

/// Used for playground stdout + stderr, or godbolt asm + stderr
/// If the return value is empty, returns " " instead, because Discord displays those better in
/// a code block than "".
#[must_use]
pub fn merge_output_and_errors<'a>(
	output: &'a str,
	errors: &'a str,
) -> std::borrow::Cow<'a, str> {
	match (output.trim(), errors.trim()) {
		("", "") => " ".into(),
		(output, "") => output.into(),
		("", errors) => errors.into(),
		(output, errors) => format!("{errors}\n\n{output}").into(),
	}
}

/// In prefix commands, react with a red cross emoji. In slash commands, respond with a short
/// explanation.
pub async fn acknowledge_fail(
	error: poise::FrameworkError<'_, BotData, Error>,
) {
	if let poise::FrameworkError::Command { error, ctx, .. } = error {
		log::warn!("Reacting with red cross because of error: {}", error);

		match ctx {
			Context::Application(_) => {
				if let Err(e) = ctx.say(format!("❌ {error}")).await {
					log::warn!(
						"Failed to send failure acknowledgment slash \
						 command response: {}",
						e
					);
				}
			}
			Context::Prefix(prefix_context) => {
				if let Err(e) = prefix_context
					.msg
					.react(ctx, serenity::ReactionType::from('❌'))
					.await
				{
					log::warn!("Failed to react with red cross: {}", e);
				}
			}
		}
	} else {
		// crate::on_error(error).await;
	}
}

#[must_use]
pub fn find_custom_emoji(
	ctx: Context<'_>,
	emoji_name: &str,
) -> Option<serenity::Emoji> {
	ctx.guild_id()?
		.to_guild_cached(&ctx)?
		.emojis
		.values()
		.find(|emoji| emoji.name.eq_ignore_ascii_case(emoji_name))
		.cloned()
}

#[must_use]
pub fn custom_emoji_code(
	ctx: Context<'_>,
	emoji_name: &str,
	fallback: char,
) -> String {
	match find_custom_emoji(ctx, emoji_name) {
		Some(emoji) => emoji.to_string(),
		None => fallback.to_string(),
	}
}

/// In prefix commands, react with a custom emoji from the guild, or fallback to a default Unicode
/// emoji.
///
/// In slash commands, currently nothing happens.
pub async fn acknowledge_success(
	ctx: Context<'_>,
	emoji_name: &str,
	fallback: char,
) -> Result<(), Error> {
	let emoji = find_custom_emoji(ctx, emoji_name);
	match ctx {
		Context::Prefix(prefix_context) => {
			let reaction = emoji.map_or_else(
				|| serenity::ReactionType::from(fallback),
				serenity::ReactionType::from,
			);

			prefix_context.msg.react(&ctx, reaction).await?;
		}
		Context::Application(_) => {
			let msg_content = match emoji {
				Some(e) => e.to_string(),
				None => fallback.to_string(),
			};
			if let Ok(reply) = ctx.say(msg_content).await {
				tokio::time::sleep(std::time::Duration::from_secs(3)).await;
				let msg = reply.message().await?;
				// ignore errors as to not fail if ephemeral
				let _: Result<_, _> = msg.delete(&ctx).await;
			}
		}
	}
	Ok(())
}

/// Truncates the message with a given truncation message if the
/// text is too long. "Too long" means, it either goes beyond Discord's 2000 char message limit,
/// or if the `text_body` has too many lines.
///
/// Only `text_body` is truncated. `text_end` will always be appended at the end. This is useful
/// for example for large code blocks. You will want to truncate the code block contents, but the
/// finalizing triple backticks (` ` `) should always stay - that's what `text_end` is for.
#[expect(clippy::doc_markdown)] // backticks cause clippy to freak out
pub async fn trim_text(
	text_body: &str,
	text_end: &str,
	truncation_msg_future: impl std::future::Future<Output = String>,
) -> String {
	const MAX_OUTPUT_LINES: usize = 45;
	const MAX_OUTPUT_LENGTH: usize = 2000;

	let needs_truncating = text_body.len() + text_end.len()
		> MAX_OUTPUT_LENGTH
		|| text_body.lines().count() > MAX_OUTPUT_LINES;

	if needs_truncating {
		let truncation_msg = truncation_msg_future.await;

		// truncate for length
		let text_body: String = text_body
			.chars()
			.take(MAX_OUTPUT_LENGTH - truncation_msg.len() - text_end.len())
			.collect();

		// truncate for lines
		let text_body = text_body
			.lines()
			.take(MAX_OUTPUT_LINES)
			.collect::<Vec<_>>()
			.join("\n");

		format!("{text_body}{text_end}{truncation_msg}")
	} else {
		format!("{text_body}{text_end}")
	}
}

pub async fn reply_potentially_long_text(
	ctx: Context<'_>,
	text_body: &str,
	text_end: &str,
	truncation_msg_future: impl std::future::Future<Output = String>,
) -> Result<(), Error> {
	ctx.say(trim_text(text_body, text_end, truncation_msg_future).await)
		.await?;
	Ok(())
}

pub async fn is_file_larger_than_mb(
	url: &str,
	max_size_mb: u64,
) -> Result<(bool, u64)> {
	let client = reqwest::Client::new();
	let response = client.head(url).send().await?;

	println!("{:#?}", response.headers());

	if let Some(content_length) =
		response.headers().get(header::CONTENT_LENGTH)
	{
		if let Ok(size) =
			content_length.to_str().unwrap_or("0").parse::<u64>()
		{
			Ok((size > max_size_mb * 1024 * 1024, size))
		} else {
			Err(anyhow!("Could not parse content-length header"))
		}
	} else {
		Err(anyhow!("Content-Length header not found"))
	}
}

pub fn change_extension<P: AsRef<Path>>(path: P, new_ext: &str) -> PathBuf {
	let mut new_path = path.as_ref().to_path_buf();

	// Remove the current extension (if any)
	new_path.set_extension(new_ext);

	new_path
}

pub fn escape_markdown(text: &str) -> String {
	let mut escaped = String::new();
	for c in text.chars() {
		// args.text.replace(/(`|\*|_|>|<)/g, "\\$1")
		match c {
			'`' => escaped.push_str("\\`"),
			'*' => escaped.push_str("\\*"),
			'_' => escaped.push_str("\\_"),
			'<' => escaped.push_str("\\<"),
			'>' => escaped.push_str("\\>"),
			'\\' => escaped.push_str("\\\\"),
			_ => escaped.push(c),
		}
	}
	escaped
}
