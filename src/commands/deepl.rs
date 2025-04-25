use std::{fs, io::Write, str::FromStr};

use deepl::{DeepLApi, DocumentStatusResp, DocumentTranslateStatus, Lang};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use num_format::ToFormattedString;
use secrecy::ExposeSecret;
use tempfile::NamedTempFile;

use crate::config::{ApplicationContext, Context};
use anyhow::{anyhow, Error, Result};
use inline_format::format;
use poise::{
	serenity_prelude::{
		Attachment, AutocompleteChoice, CreateActionRow,
		CreateAllowedMentions, CreateAttachment, CreateAutocompleteResponse,
		CreateButton, CreateComponent, CreateContainer, CreateFile,
		CreateSeparator, CreateTextDisplay, CreateUnfurledMediaItem, Message,
		MessageFlags,
	},
	CreateReply, Modal,
};

pub const ACCENT_COLOR: u32 = 0x0f2b46;

pub const LANGUAGES: &'static [Lang; 36] = &[
	Lang::AR,
	Lang::BG,
	Lang::CS,
	Lang::DA,
	Lang::DE,
	Lang::EL,
	Lang::EN,
	Lang::EN_GB,
	Lang::EN_US,
	Lang::ES,
	Lang::ET,
	Lang::FI,
	Lang::FR,
	Lang::HU,
	Lang::ID,
	Lang::IT,
	Lang::JA,
	Lang::KO,
	Lang::LT,
	Lang::LV,
	Lang::NB,
	Lang::NL,
	Lang::PL,
	Lang::PT,
	Lang::PT_BR,
	Lang::PT_PT,
	Lang::RO,
	Lang::RU,
	Lang::SK,
	Lang::SL,
	Lang::SV,
	Lang::TR,
	Lang::UK,
	Lang::ZH,
	Lang::ZH_HANS,
	Lang::ZH_HANT,
];

pub async fn fuzzy_langs<'a>(partial: &'a str) -> Vec<Lang> {
	let mut response = vec![];

	let matcher = SkimMatcherV2::default();

	// Sort by averaged score.
	// Based on language name and code.
	let mut target_langs = LANGUAGES
		.clone()
		.map(|lang| {
			(
				lang.clone(),
				(matcher
					.fuzzy_match(&lang.description(), partial)
					.unwrap_or(0) + matcher
					.fuzzy_match(&lang.to_string(), partial)
					.unwrap_or(0)) / 2,
			)
		})
		.to_vec();

	// If all are 0, return the first 25.
	if target_langs.iter().all(|(_, score)| *score == 0) {
		target_langs = LANGUAGES
			.iter()
			.take(25)
			.map(|lang| (lang.clone(), 0i64))
			.collect();
	} else {
		// Otherwise remove all 0s...
		target_langs = target_langs
			.clone()
			.iter()
			.filter(|(_, score)| *score > 0)
			.cloned()
			.collect();
		// Sort by code‐score, then by name‐score.
		target_langs.sort_by_key(|(_, score)| *score);
		target_langs.reverse();
	}

	// Take 25 at most for Discord's limits.
	target_langs.truncate(25);

	for (lang, ..) in target_langs {
		response.push(lang.clone());
	}

	response
}

async fn autocomplete_deepl_language<'a>(
	_ctx: Context<'a>,
	partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
	let mut response = CreateAutocompleteResponse::new();
	let langs = fuzzy_langs(partial).await;

	for lang in langs {
		response = response.add_choice(AutocompleteChoice::new(
			lang.description(),
			lang.to_string(),
		));
	}

	response
}

fn create_deepl_link(
	ctx: &Context<'_>,
	text: &impl ToString,
	target_lang: &Lang,
	source_lang: &Lang,
) -> String {
	format!(
		"https://www.deepl.com/",
		ctx.data()
			.config
			.deepl_default_target_language
			.clone()
			.unwrap_or(Lang::EN)
			.to_string()
			.to_lowercase(),
		"/translator#",
		source_lang.to_string().to_lowercase(),
		"/",
		target_lang.to_string().to_lowercase(),
		"/",
		urlencoding::encode(text.to_string().as_str())
	)
}

pub async fn create_translation_reply<'a>(
	ctx: &Context<'a>,
	reply: CreateReply<'a>,
	text: String,
	target_language: Lang,
	source_language: Option<Lang>,
) -> Result<CreateReply<'a>> {
	let mut reply = reply.clone();

	let api = make_deepl_api(&ctx)?;

	let mut translation_result =
		api.translate_text(&text, target_language.clone());
	if let Some(source_language) = source_language {
		translation_result.source_lang(source_language);
	}
	let translation_result = translation_result.await?;

	if let Some(first_sentence) = translation_result.translations.first() {
		let source_language = first_sentence.detected_source_language.clone();
		let deepl_link = create_deepl_link(
			&ctx,
			&text,
			&target_language,
			&source_language,
		);
		let mut components = [
			CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
				"## From ",
				source_language.description(),
				"\n",
				text,
			))),
			CreateComponent::Separator(CreateSeparator::new(true)),
			CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
				"## To ",
				target_language.description(),
				"\n",
				translation_result
			))),
		]
		.to_vec()
		.to_owned();
		// The max length of a link in a button in Discord is 512 characters.
		if deepl_link.len() <= 512 {
			components
				.push(CreateComponent::Separator(CreateSeparator::new(true)));
			components.push(CreateComponent::ActionRow(
				CreateActionRow::Buttons(
					[CreateButton::new_link(deepl_link).label("View Online")]
						.to_vec()
						.into(),
				),
			));
		}
		reply = reply.flags(MessageFlags::IS_COMPONENTS_V2);
		reply = reply.components(
			[CreateComponent::Container(
				CreateContainer::new(components).accent_color(ACCENT_COLOR),
			)]
			.to_vec()
			.to_owned(),
		);
	} else {
		return Err(anyhow!("Translation was empty."));
	}

	Ok(reply)
}

#[derive(Debug, Modal)]
#[name = "Translate Message"]
struct TranslateModal {
	#[name = "Target Language"]
	#[placeholder = "The language to translate to. (default: config language)"]
	target_language: Option<String>,
	#[name = "Source Language"]
	#[placeholder = "The language to translate from. (default: detect language)"]
	source_language: Option<String>,
	#[placeholder = "Whether or not to show the message."]
	ephemeral: Option<String>,
}

/// Translates text using DeepL.
#[poise::command(
	context_menu_command = "Translate Message",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn translate_context_menu(
	ctx: ApplicationContext<'_>,
	#[description = "The message to convert to WebP."] message: Message,
) -> Result<()> {
	let data = TranslateModal::execute(ctx)
		.await?
		.ok_or_else(|| anyhow!("No modal data."))?;

	let ephemeral = match data.ephemeral.as_deref() {
		Some("false") => false,
		Some(_) => true,
		None => false,
	};

	let text = message.content.to_string();

	if text.is_empty() {
		return Err(anyhow!("No text to translate."));
	}

	let target_language =
		if let Some(target_language) = data.target_language.as_deref() {
			Lang::from_str(&target_language)?
		} else {
			Lang::from_str(
				ctx.data()
					.config
					.deepl_default_target_language
					.clone()
					.unwrap_or(Lang::EN)
					.to_string()
					.as_str(),
			)?
		};
	let source_language =
		if let Some(source_language) = data.source_language.as_deref() {
			Some(Lang::from_str(&source_language)?)
		} else {
			None
		};

	let reply = create_translation_reply(
		&poise::Context::Application(ctx),
		CreateReply::default()
			.allowed_mentions(CreateAllowedMentions::default())
			.ephemeral(ephemeral),
		text,
		target_language,
		source_language,
	)
	.await?;

	ctx.send(reply).await?;

	Ok(())
}

pub fn make_deepl_api(ctx: &Context<'_>) -> Result<DeepLApi> {
	if let Some(deepl_api_key) = ctx.data().config.deepl_api_key.as_ref() {
		Ok(DeepLApi::with(deepl_api_key.clone().expose_secret()).new())
	} else {
		Err(anyhow!("Missing DeepL API key."))
	}
}

/// A collection of commands for DeepL.
#[poise::command(
	slash_command,
	subcommands("deepl_translate", "deepl_usage"),
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn deepl(_ctx: Context<'_>, _arg: String) -> Result<()> {
	Ok(())
}

/// A collection of commands for DeepL.
#[poise::command(
	rename = "translate",
	slash_command,
	subcommands("deepl_translate_text", "deepl_translate_document"),
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn deepl_translate(_ctx: Context<'_>, _arg: String) -> Result<()> {
	Ok(())
}

/// Translates text from one language to another.
#[poise::command(
	rename = "text",
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn deepl_translate_text(
	ctx: Context<'_>,
	#[description = "The text to translate."] text: String,
	#[description = "The target language code to translate to."]
	#[autocomplete = "autocomplete_deepl_language"]
	target_language: Option<String>,
	#[description = "The source language code to translate from."]
	#[autocomplete = "autocomplete_deepl_language"]
	source_language: Option<String>,
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

	let target_language = if let Some(target_language) = target_language {
		Lang::from_str(&target_language)?
	} else {
		Lang::from_str(
			ctx.data()
				.config
				.deepl_default_target_language
				.clone()
				.unwrap_or(Lang::EN)
				.to_string()
				.as_str(),
		)?
	};
	let source_language = if let Some(source_language) = source_language {
		Some(Lang::from_str(&source_language)?)
	} else {
		None
	};

	let reply = create_translation_reply(
		&ctx,
		CreateReply::default()
			.allowed_mentions(CreateAllowedMentions::default())
			.ephemeral(ephemeral),
		text,
		target_language,
		source_language,
	)
	.await?;

	ctx.send(reply).await?;

	Ok(())
}

pub enum DeepLDocumentTranslationStatus<'a> {
	Uploading,
	Translating(Option<DocumentStatusResp>),
	Sending,
	Finished(DocumentStatusResp, Option<Lang>, Lang, CreateAttachment<'a>),
	Error(Error),
}
pub fn make_document_translation_reply<'a>(
	ctx: &Context<'a>,
	status: DeepLDocumentTranslationStatus<'a>,
	ephemeral: bool,
) -> Result<CreateReply<'a>> {
	Ok(match status {
		DeepLDocumentTranslationStatus::Uploading => CreateReply::new()
			.allowed_mentions(CreateAllowedMentions::default())
			.flags(MessageFlags::IS_COMPONENTS_V2)
			.components(
				[CreateComponent::Container(
					CreateContainer::new(
						[CreateComponent::TextDisplay(
							CreateTextDisplay::new(format!(
							"## Uploading <a:loading:1365387946127130744>"
						)),
						)]
						.to_vec()
						.to_owned(),
					)
					.accent_color(ACCENT_COLOR),
				)]
				.to_vec()
				.to_owned(),
			),
		DeepLDocumentTranslationStatus::Translating(None) => {
			CreateReply::new()
				.allowed_mentions(CreateAllowedMentions::default())
				.flags(MessageFlags::IS_COMPONENTS_V2)
				.components(
					[CreateComponent::Container(
						CreateContainer::new(
							[CreateComponent::TextDisplay(
								CreateTextDisplay::new(format!(
							"## Translating <a:loading:1365387946127130744>"
						)),
							)]
							.to_vec()
							.to_owned(),
						)
						.accent_color(ACCENT_COLOR),
					)]
					.to_vec()
					.to_owned(),
				)
		}
		DeepLDocumentTranslationStatus::Translating(Some(status)) => {
			let status_name = match status.status {
				DocumentTranslateStatus::Done => "Done",
				DocumentTranslateStatus::Translating => "In Progress",
				DocumentTranslateStatus::Queued => "Queued",
				DocumentTranslateStatus::Error => "Error",
			};
			let billed_characters = status
				.billed_characters
				.unwrap_or(0)
				.to_formatted_string(&*ctx.data().config.locale);
			let seconds_remaining = status
				.seconds_remaining
				.unwrap_or(0)
				.to_formatted_string(&*ctx.data().config.locale);
			CreateReply::new()
				.allowed_mentions(CreateAllowedMentions::default())
				.flags(MessageFlags::IS_COMPONENTS_V2)
				.components(
					[CreateComponent::Container(
						CreateContainer::new(
							[CreateComponent::TextDisplay(
								CreateTextDisplay::new(format!(
									"## Translating ",
									status_name,
									" <a:loading:1365387946127130744>\n",
									billed_characters,
									" characters.\n",
									seconds_remaining,
									" seconds remaining."
								)),
							)]
							.to_vec()
							.to_owned(),
						)
						.accent_color(ACCENT_COLOR),
					)]
					.to_vec()
					.to_owned(),
				)
		}
		DeepLDocumentTranslationStatus::Sending => CreateReply::new()
			.allowed_mentions(CreateAllowedMentions::default())
			.flags(MessageFlags::IS_COMPONENTS_V2)
			.components(
				[CreateComponent::Container(
					CreateContainer::new(
						[CreateComponent::TextDisplay(
							CreateTextDisplay::new(format!(
								"## Sending <a:loading:1365387946127130744>"
							)),
						)]
						.to_vec()
						.to_owned(),
					)
					.accent_color(ACCENT_COLOR),
				)]
				.to_vec()
				.to_owned(),
			),
		DeepLDocumentTranslationStatus::Error(error) => CreateReply::new()
			.allowed_mentions(CreateAllowedMentions::default())
			.flags(MessageFlags::IS_COMPONENTS_V2)
			.components(
				[CreateComponent::Container(
					CreateContainer::new(
						[CreateComponent::TextDisplay(
							CreateTextDisplay::new(format!(
								"## Error\n",
								error.to_string()
							)),
						)]
						.to_vec()
						.to_owned(),
					)
					.accent_color(ACCENT_COLOR),
				)]
				.to_vec()
				.to_owned(),
			),
		DeepLDocumentTranslationStatus::Finished(
			status,
			source_language,
			target_language,
			attachment,
		) => {
			let billed_characters = status
				.billed_characters
				.unwrap_or(0)
				.to_formatted_string(&*ctx.data().config.locale);
			CreateReply::new()
				.allowed_mentions(CreateAllowedMentions::default())
				.flags(MessageFlags::IS_COMPONENTS_V2)
				.components(
					[CreateComponent::Container(
						CreateContainer::new(
							[
								CreateComponent::TextDisplay(
									CreateTextDisplay::new(format!(
										"## __",
										match source_language {
											Some(lang) => {
												lang.description()
											}
											None =>
												"Detect Language".to_string(),
										},
										"__ To __",
										target_language.description(),
										"__\n",
										billed_characters,
										" characters."
									)),
								),
								CreateComponent::File(CreateFile::new(
									CreateUnfurledMediaItem::new(format!(
										"attachment://",
										attachment.filename
									)),
								)),
							]
							.to_vec()
							.to_owned(),
						)
						.accent_color(ACCENT_COLOR),
					)]
					.to_vec()
					.to_owned(),
				)
				.attachment(attachment)
		}
	}
	.ephemeral(ephemeral))
}

/// Translates a document from one language to another.
#[poise::command(
	rename = "document",
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn deepl_translate_document(
	ctx: Context<'_>,
	#[description = "The document to translate."] document: Attachment,
	#[description = "The target language code to translate to."]
	#[autocomplete = "autocomplete_deepl_language"]
	target_language: Option<String>,
	#[description = "The source language code to translate from."]
	#[autocomplete = "autocomplete_deepl_language"]
	source_language: Option<String>,
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

	let target_language = if let Some(target_language) = target_language {
		Lang::from_str(&target_language)?
	} else {
		Lang::from_str(
			ctx.data()
				.config
				.deepl_default_target_language
				.clone()
				.unwrap_or(Lang::EN)
				.to_string()
				.as_str(),
		)?
	};
	let source_language = if let Some(source_language) = source_language {
		Some(Lang::from_str(&source_language)?)
	} else {
		None
	};

	let reply = make_document_translation_reply(
		&ctx,
		DeepLDocumentTranslationStatus::Uploading,
		ephemeral,
	)?;

	let message = ctx.send(reply).await?;

	// Download the document
	let document_bytes = document.download().await?;
	// Save to temp file.
	let mut temp_file = NamedTempFile::new()?;
	temp_file.write(&document_bytes)?;

	let api = make_deepl_api(&ctx)?;

	let mut upload =
		api.upload_document(temp_file.path(), target_language.clone());
	let mut upload = upload.filename(document.filename.to_string());
	if let Some(source_language) = source_language.clone() {
		upload = upload.source_lang(source_language);
	}
	let upload = upload.await;

	let upload = match upload {
		Ok(upload) => upload,
		Err(error) => {
			message
				.edit(
					ctx,
					make_document_translation_reply(
						&ctx,
						DeepLDocumentTranslationStatus::Error(error.into()),
						ephemeral,
					)?,
				)
				.await?;
			return Ok(());
		}
	};

	message
		.edit(
			ctx,
			make_document_translation_reply(
				&ctx,
				DeepLDocumentTranslationStatus::Translating(None),
				ephemeral,
			)?,
		)
		.await?;

	// Loop until it's translated.
	loop {
		let status = api.check_document_status(&upload).await?;
		if let Some(error_message) = status.error_message {
			message
				.edit(
					ctx,
					make_document_translation_reply(
						&ctx,
						DeepLDocumentTranslationStatus::Error(anyhow!(
							error_message
						)),
						ephemeral,
					)?,
				)
				.await?;
			return Ok(());
		}
		if status.status == DocumentTranslateStatus::Done {
			message
				.edit(
					ctx,
					make_document_translation_reply(
						&ctx,
						DeepLDocumentTranslationStatus::Sending,
						ephemeral,
					)?,
				)
				.await?;

			let temp_file = NamedTempFile::new()?;
			api.download_document(&upload, temp_file.path()).await?;
			let download_bytes = fs::read(temp_file)?;

			let attachment = CreateAttachment::bytes(
				download_bytes,
				document.filename.to_string(),
			);

			message
				.edit(
					ctx,
					make_document_translation_reply(
						&ctx,
						DeepLDocumentTranslationStatus::Finished(
							status,
							source_language,
							target_language,
							attachment,
						),
						ephemeral,
					)?,
				)
				.await?;

			return Ok(());
		}

		message
			.edit(
				ctx,
				make_document_translation_reply(
					&ctx,
					DeepLDocumentTranslationStatus::Translating(Some(status)),
					ephemeral,
				)?,
			)
			.await?;
		// Do not spam the API.
		std::thread::sleep(std::time::Duration::from_secs(1));
	}
}

/// Displays DeepL API usage information.
#[poise::command(
	rename = "usage",
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn deepl_usage(
	ctx: Context<'_>,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let ephemeral = ephemeral.unwrap_or(true);
	if ephemeral {
		ctx.defer_ephemeral().await?;
	} else {
		ctx.defer().await?;
	}

	let api = make_deepl_api(&ctx)?;

	let response = api.get_usage().await?;
	// Format the two numbers into a string with commas.
	let formatted_usage = format!(
		response
			.character_count
			.to_formatted_string(&*ctx.data().config.locale),
		" / ",
		response
			.character_limit
			.to_formatted_string(&*ctx.data().config.locale),
		" characters."
	);

	let reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral)
		.content(formatted_usage);

	ctx.send(reply).await?;

	Ok(())
}
