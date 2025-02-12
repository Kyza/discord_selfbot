use anyhow::{anyhow, Result};
use byte_unit::rust_decimal::prelude::ToPrimitive;
use inline_format::format;
use serde::Deserialize;
use url::Url;

use crate::{commands::build_song_info_message, config::Context};

#[derive(Debug, Clone)]
pub struct PlayingNow {
	pub user_id: String,
	pub playing_now: bool,
	pub duration: Option<u32>,
	pub origin_url: Option<String>,
	pub artist_name: Option<String>,
	pub track_name: Option<String>,
}

impl<'de> Deserialize<'de> for PlayingNow {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value =
			serde_json::Value::deserialize(deserializer)?["payload"].clone();

		Ok(Self {
			user_id: value["user_id"].as_str().unwrap().to_string(),
			playing_now: value["playing_now"].as_bool().unwrap(),
			duration: value["listens"][0]["track_metadata"]
				["additional_info"]["duration"]
				.as_u64()
				.map(|s| s.to_u32())
				.flatten(),
			origin_url: value["listens"][0]["track_metadata"]
				["additional_info"]["origin_url"]
				.as_str()
				.map(|s| s.to_string()),
			artist_name: value["listens"][0]["track_metadata"]["artist_name"]
				.as_str()
				.map(|s| s.to_string()),
			track_name: value["listens"][0]["track_metadata"]["track_name"]
				.as_str()
				.map(|s| s.to_string()),
		})
	}
}

/// Shows what you're currently listening to from the ListenBrainz API.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn now_playing(
	ctx: Context<'_>,
	#[description = "The user to show the currently playing song of."]
	user: Option<String>,
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

	let playing_now_data = ctx
		.data()
		.http
		.get(format!(
			"https://api.listenbrainz.org/1/user/",
			&urlencoding::encode(
				&user.or(
					ctx.data().config.listenbrainz_user.clone()
				).ok_or(
					anyhow!("No user was provided and no default user is set in the config.")
				)?
			),
			"/playing-now"
		))
		.send()
		.await?
		.json::<PlayingNow>()
		.await?;

	if playing_now_data.origin_url.is_none() {
		return Err(anyhow!("No song is currently playing."));
	}

	ctx.send(
		build_song_info_message(
			&ctx,
			Url::parse(&playing_now_data.origin_url.ok_or(anyhow!(
				"The song link from ListenBrainz isn't a valid URL."
			))?)?,
			playing_now_data.track_name.clone(),
			playing_now_data.artist_name.clone(),
			ephemeral,
		)
		.await?,
	)
	.await?;
	Ok(())
}
