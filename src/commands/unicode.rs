use anyhow::Result;
use poise::{serenity_prelude::CreateAllowedMentions, CreateReply};

use crate::types::Context;

/// Converts text to and from Unicode.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn unicode(
	ctx: Context<'_>,
	#[description = "Convert a string to Unicode."] to_unicode: Option<
		String,
	>,
	#[description = "Convert some Unicode to a string."] from_unicode: Option<
		String,
	>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let text = if let Some(input) = to_unicode {
		// Convert to Unicode
		input
			.chars()
			.map(|c| format!("U+{:04X}", c as u32))
			.collect::<Vec<String>>()
			.join(" ")
	} else if let Some(input) = from_unicode {
		// Convert from Unicode
		input
			.split_whitespace()
			.filter_map(|s| {
				u32::from_str_radix(&s.trim_start_matches("U+"), 16).ok()
			})
			.filter_map(char::from_u32)
			.collect::<String>()
	} else {
		"Please provide either \"to_unicode\" or \"from_unicode\" parameter."
			.to_string()
	};

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.content(format!("```\n{}\n```", text))
		.ephemeral(ephemeral.unwrap_or(false));

	ctx.send(reply).await?;
	Ok(())
}
