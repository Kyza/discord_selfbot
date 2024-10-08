use anyhow::Result;
use poise::{
	serenity_prelude::CreateAllowedMentions, ChoiceParameter, CreateReply,
};
use serde::{Deserialize, Serialize};

use crate::types::Context;

#[derive(Debug, Serialize, Deserialize, Clone, ChoiceParameter)]
pub enum WaybackAction {
	Latest,
	Overview,
	Save,
}

const BASE_URL: &str = "https://web.archive.org";

/// Generates an archive.org (Wayback Machine) URL for a given URL.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn wayback(
	ctx: Context<'_>,
	#[description = "Target URL."] url: url::Url,
	#[description = "The action the URL should perform."] action: Option<
		WaybackAction,
	>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let url = urlencoding::encode(url.as_str());
	let response = match action.unwrap_or(WaybackAction::Latest) {
		WaybackAction::Latest => {
			format!("{}/web/{}", BASE_URL, url)
		}
		WaybackAction::Overview => {
			format!("{}/web/*/{}", BASE_URL, url)
		}
		WaybackAction::Save => format!("{}/save/{}", BASE_URL, url),
	};

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.content(response)
		.ephemeral(ephemeral.unwrap_or(true));

	ctx.send(reply).await?;
	Ok(())
}
