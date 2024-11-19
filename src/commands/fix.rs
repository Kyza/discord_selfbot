use crate::types::Context;
use anyhow::Result;
use poise::{
	serenity_prelude::{CreateAllowedMentions, CreateAttachment},
	ChoiceParameter, CreateReply,
};
use serde::{Deserialize, Serialize};
use url::{Host, Url};

#[derive(Debug, Serialize, Deserialize, Clone, ChoiceParameter)]
enum InstagramView {
	Default,
	Direct,
	Gallery,
}
impl InstagramView {
	fn to_prefix(self) -> &'static str {
		match self {
			Self::Default => "dd",
			Self::Direct => "d.dd",
			Self::Gallery => "g.dd",
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, ChoiceParameter)]
enum TikTokView {
	Default,
	Direct,
	Description,
}
impl TikTokView {
	fn to_query(self) -> Option<&'static str> {
		match self {
			Self::Default => None,
			Self::Direct => Some("?isDirect=true"),
			Self::Description => Some("?addDesc=true"),
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, ChoiceParameter)]
enum XView {
	Default,
	Gallery,
	Text,
	Direct,
}
impl XView {
	fn to_subdomain(self) -> &'static str {
		match self {
			Self::Default => "",
			Self::Gallery => "g.",
			Self::Text => "t.",
			Self::Direct => "d.",
		}
	}
}

/// Makes social media links embed properly. Works for X, TikTok, Instagram, and Reddit.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn fix(
	ctx: Context<'_>,
	#[description = "Social media URL."] url: Url,
	#[description = "[Instagram] Which view to use."] instagram_view: Option<
		InstagramView,
	>,
	#[description = "[TikTok] Which view to use."] tiktok_view: Option<
		TikTokView,
	>,
	#[description = "[X] Which view to use."] x_view: Option<XView>,
	#[description = "[X] The 2 letter language code to translate to."]
	x_language: Option<String>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let instagram_view = instagram_view.unwrap_or(InstagramView::Default);
	let tiktok_view = tiktok_view.unwrap_or(TikTokView::Default);
	let x_view = x_view.unwrap_or(XView::Default);
	let ephemeral = ephemeral.unwrap_or(false);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	match url.host() {
		Some(Host::Domain("instagram.com"))
		| Some(Host::Domain("www.instagram.com")) => {
			let mut new_url = url.clone();
			new_url.set_host(Some(&format!(
				"{}instagram.com",
				instagram_view.to_prefix()
			)))?;
			new_url.set_query(None);
			ctx.send(reply.content(new_url)).await?;
		}
		Some(Host::Domain("tiktok.com"))
		| Some(Host::Domain("www.tiktok.com")) => {
			let mut new_url = url.clone();
			new_url.set_host(Some("tnktok.com"))?;
			new_url.set_query(tiktok_view.to_query());
			ctx.send(reply.content(new_url)).await?;
		}
		Some(Host::Domain("reddit.com"))
		| Some(Host::Domain("redd.it"))
		| Some(Host::Domain("old.reddit.com"))
		| Some(Host::Domain("www.reddit.com")) => {
			let mut new_url = url.clone();
			new_url.set_host(Some("rxddit.com"))?;
			new_url.set_query(None);
			ctx.send(reply.content(new_url)).await?;
		}
		Some(Host::Domain("www.twitter.com"))
		| Some(Host::Domain("twitter.com"))
		| Some(Host::Domain("www.x.com"))
		| Some(Host::Domain("x.com")) => {
			let mut new_url = url.clone();
			new_url.set_host(Some(&format!(
				"{}fixupx.com",
				x_view.to_subdomain()
			)))?;
			new_url.set_path(&format!(
				"{}{}",
				new_url.path(),
				if let Some(language) = x_language {
					format!("/{}", language)
				} else {
					"".to_string()
				}
			));
			new_url.set_query(None);
			ctx.send(reply.content(new_url)).await?;
		}
		Some(_) | None => {
			ctx.send(reply.content("Use a real website idiot.")).await?;
		}
	}

	Ok(())
}
