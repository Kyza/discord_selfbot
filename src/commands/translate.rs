use std::time::Duration;
use thirtyfour::prelude::*;
use url::Url;

use crate::types::{ApplicationContext, Context};
use anyhow::{anyhow, Result};
use fancy_regex::Regex;
use inline_format::{format, println};
use poise::{
	serenity_prelude::{
		CreateActionRow, CreateAllowedMentions, CreateButton, Message,
	},
	CreateReply, Modal,
};

// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub enum DeepLLanguage {
// 	Auto,
// 	Arabic,
// 	Bulgarian,
// 	Chinese,
// 	ChineseSimplified,
// 	ChineseTraditional,
// 	Czech,
// 	Danish,
// 	Dutch,
// 	English,
// 	EnglishAmerican,
// 	EnglishBritish,
// 	Estonian,
// 	Finnish,
// 	French,
// 	German,
// 	Greek,
// 	Hungarian,
// 	Indonesian,
// 	Italian,
// 	Japanese,
// 	Korean,
// 	Latvian,
// 	Lithuanian,
// 	NorwegianBokmål,
// 	Polish,
// 	Portuguese,
// 	PortugueseBrazillian,
// 	Romanian,
// 	Russian,
// 	Slovak,
// 	Slovenian,
// 	Spanish,
// 	Swedish,
// 	Turkish,
// 	Ukrainian,
// }
// #[async_trait]
// impl SlashArgument for DeepLLanguage {
// 	async fn extract(
// 		_: &serenity::Context,
// 		_: &serenity::CommandInteraction,
// 		value: &serenity::ResolvedValue<'_>,
// 	) -> Result<DeepLLanguage, SlashArgError> {
// 		match *value {
// 			serenity::ResolvedValue::String(x) => {
// 				serde_plain::from_str::<DeepLLanguage>(x).map_err(|_| {
// 					SlashArgError::new_command_structure_mismatch(
// 						"received invalid quality",
// 					)
// 				})
// 			}
// 			_ => Err(SlashArgError::new_command_structure_mismatch(
// 				"expected string",
// 			)),
// 		}
// 	}

// 	fn create(
// 		builder: serenity::CreateCommandOption,
// 	) -> serenity::CreateCommandOption {
// 		builder
// 			.add_string_choice("Auto", "auto")
// 			.add_string_choice("Arabic", "ar")
// 			.add_string_choice("Bulgarian", "bg")
// 			.add_string_choice("Chinese Simplified", "hans")
// 			.add_string_choice("Chinese Traditional", "hant")
// 			.add_string_choice("Czech", "cs")
// 			.add_string_choice("Danish", "da")
// 			.add_string_choice("Dutch", "nl")
// 			.add_string_choice("English (American)", "en-US")
// 			.add_string_choice("English (British)", "en-GB")
// 			.add_string_choice("Estonian", "et")
// 			.add_string_choice("Finnish", "fi")
// 			.add_string_choice("French", "fr")
// 			.add_string_choice("German", "de")
// 			.add_string_choice("Greek", "el")
// 			.add_string_choice("Hungarian", "hu")
// 			.add_string_choice("Indonesian", "id")
// 			.add_string_choice("Italian", "it")
// 			.add_string_choice("Japanese", "ja")
// 			.add_string_choice("Korean", "ko")
// 			.add_string_choice("Latvian", "lv")
// 			.add_string_choice("Lithuanian", "lt")
// 			.add_string_choice("Norwegian (bokmål)", "nb")
// 			.add_string_choice("Polish", "pl")
// 			.add_string_choice("Portuguese", "pt")
// 			.add_string_choice("Portuguese (Brazillian)", "pt-BR")
// 			.add_string_choice("Romanian", "ro")
// 			.add_string_choice("Russian", "ru")
// 			.add_string_choice("Slovak", "sk")
// 			.add_string_choice("Slovenian", "sl")
// 			.add_string_choice("Spanish", "es")
// 			.add_string_choice("Swedish", "sv")
// 			.add_string_choice("Turkish", "tr")
// 			.add_string_choice("Ukrainian", "uk")
// 	}
// }

#[derive(Debug, Modal)]
#[name = "Translate Message"]
struct TranslateModal {
	#[name = "Target Language"]
	#[placeholder = "The language to translate to. (default: your language)"]
	target_language: Option<String>,
	#[name = "Source Language"]
	#[placeholder = "The language to translate from. (default: detect)"]
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

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral);

	let content =
		get_content_from_message(&Context::from(ctx), &message).await?;

	let translation_result = translate_text(
		&content,
		&data.source_language.unwrap_or("auto".to_string()),
		&data
			.target_language
			.unwrap_or(ctx.data().config.deepl_target_language.clone()),
	)
	.await?;

	reply = reply.components(vec![CreateActionRow::Buttons(vec![
		CreateButton::new_link(translation_result.url).label("View Online"),
	])]);
	reply = reply.content(translation_result.translated_text);

	ctx.send(reply).await?;

	Ok(())
}

async fn get_content_from_message(
	ctx: &Context<'_>,
	message: &Message,
) -> Result<String> {
	if let Some(reference) = message.message_reference.clone() {
		// Get it from the IDs.
		let message = ctx
			.http()
			.get_message(reference.channel_id, reference.message_id.unwrap())
			.await?;
		Ok(message.content.clone())
	} else {
		Ok(message.content.clone())
	}
}

/// Translates text from one language to another.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn translate(
	ctx: Context<'_>,
	#[description = "The text to translate."] text: String,
	#[description = "The source language code to translate from."]
	source_language: Option<String>,
	#[description = "The target language code to translate to."]
	target_language: Option<String>,
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

	let translation_result = translate_text(
		&text,
		&source_language.unwrap_or("auto".to_string()),
		&target_language
			.unwrap_or(ctx.data().config.deepl_target_language.clone()),
	)
	.await?;

	reply = reply.components(vec![CreateActionRow::Buttons(vec![
		CreateButton::new_link(translation_result.url).label("View Online"),
	])]);
	reply = reply.content(translation_result.translated_text);

	ctx.send(reply).await?;

	Ok(())
}

pub async fn wait_for_element(
	driver: &WebDriver,
	selector: &str,
) -> Result<WebElement> {
	// Sometimes the element becomes stale instantly. No idea why. Cry about it.
	while let Err(_) = driver
		.query(By::Css(selector))
		.wait(Duration::from_secs(60), Duration::from_millis(10))
		.first()
		.await
	{}
	Ok(driver
		.query(By::Css(selector))
		.wait(Duration::from_secs(60), Duration::from_millis(10))
		.first()
		.await?)
}

pub struct TranslationResult {
	pub translated_text: String,
	pub url: Url,
	pub source_language: String,
	pub target_language: String,
	pub auto_detected_source_language: bool,
}

pub async fn translate_text(
	text: &String,
	source_language: &String,
	target_language: &String,
) -> Result<TranslationResult> {
	println!("Starting Firefox.");

	let mut caps = DesiredCapabilities::firefox();
	// If in debug mode, run non-headless.
	// Look at him he has Smitty Werbenjägermanjensen's hat.
	if !cfg!(debug_assertions) {
		caps.set_headless()?;
	}
	// caps.set_log_level(LogLevel::Trace)?;
	let driver = WebDriver::new("http://localhost:4444", caps).await?;

	let starting_url = format!("https://www.deepl.com/en/translator");
	driver.goto(starting_url.clone()).await?;

	// The page is done loading when the glossary button is clickable.
	let glossary_button =
		wait_for_element(&driver, "[data-testid='glossary-button']").await?;
	glossary_button.wait_until().clickable().await?;

	println!("Page loaded.");

	let source_element = wait_for_element(
		&driver,
		"d-textarea[name='source'] [contenteditable='true']",
	)
	.await?;
	source_element.focus().await?;
	source_element.send_keys(text).await?;

	let mut attempted_source_language = "auto".to_string();
	if source_language != "auto" {
		let source_selector_element = driver
			.find(By::Css("[data-testid='translator-source-lang-btn']"))
			.await?;
		source_selector_element.wait_until().clickable().await?;
		source_selector_element.click().await?;

		let source_list_search_element = wait_for_element(
			&driver,
			"[data-testid='translator-source-lang-list'] input",
		)
		.await?;
		source_list_search_element
			.send_keys(source_language)
			.await?;
		let source_lang_option_element = wait_for_element(&driver, "[data-testid='translator-source-lang-list'] [class*='grid-cols-1'] [data-testid^='translator-lang-option-']:not([data-testid='translator-lang-option-auto'])")
		.await?;
		attempted_source_language = source_lang_option_element
			.attr("data-testid")
			.await?
			.unwrap_or("translator-lang-option-unknown".to_string())
			.replace("translator-lang-option-", "")
			.to_lowercase();
		source_lang_option_element.click().await?;
	}

	let target_selector_element = driver
		.find(By::Css("[data-testid='translator-target-lang-btn']"))
		.await?;
	target_selector_element.wait_until().clickable().await?;
	target_selector_element.click().await?;

	let target_list_search_element = wait_for_element(
		&driver,
		"[data-testid='translator-target-lang-list'] input",
	)
	.await?;
	target_list_search_element
		.send_keys(target_language)
		.await?;
	let target_lang_option_element = wait_for_element(&driver, "[data-testid='translator-target-lang-list'] [class*='grid-cols-1'] [data-testid^='translator-lang-option-']:not([data-testid='translator-lang-option-auto'])")
		.await?;
	let attempted_target_language = target_lang_option_element
		.attr("data-testid")
		.await?
		.unwrap_or("translator-lang-option-unknown".to_string())
		.replace("translator-lang-option-", "")
		.to_lowercase();
	target_lang_option_element.click().await?;

	// Wait for the element to contain text.
	let target_element =
		wait_for_element(&driver, "d-textarea[name='target']").await?;
	while target_element.text().await.is_err()
		|| target_element.text().await?.trim() == ""
	{
		tokio::time::sleep(Duration::from_millis(10)).await;
	}
	let translated_text = target_element.text().await?.trim().to_string();

	// Get selected language.
	println!(starting_url);
	while driver.current_url().await?.as_str() == starting_url {
		tokio::time::sleep(Duration::from_millis(10)).await;
	}
	let mut url = driver.current_url().await?;
	// Remove the first segment of the URL if it's not "translator/".
	url.set_path(
		&url.path().split('/').skip(2).collect::<Vec<_>>().join("/"),
	);
	println!("URL: ", url:?);

	let url_regex = Regex::new(
		r"https:\/\/www\.deepl\.com\/translator(?:#(?<source>.+))?\/(?<target>.+)\/.*",
	)
	.unwrap();
	// Get the data.
	let captures = url_regex
		.captures(url.as_str())?
		.ok_or_else(|| anyhow!("Failed to get data from the URL."))?;

	// let result_target_language = captures
	// 	.name("target")
	// 	.ok_or_else(|| anyhow!("Could not find the target language."))?
	// 	.as_str()
	// 	.to_string();
	let result_source_language = captures
		.name("source")
		.ok_or_else(|| anyhow!("Could not find the source language."))?
		.as_str()
		.to_string()
		.to_lowercase();

	// Test if the previously set language is the same as the current one.
	// data-testid="translator-target-lang"
	// dl-selected-lang="..."
	let target_lang_element = driver
		.find(By::Css("[data-testid='translator-target-lang']"))
		.await?;
	let source_lang_element = driver
		.find(By::Css("[data-testid='translator-source-lang']"))
		.await?;
	let current_target_language = target_lang_element
		.attr("dl-selected-lang")
		.await?
		.unwrap_or("unknown".to_string())
		.to_lowercase();
	let current_source_language = source_lang_element
		.attr("dl-selected-lang")
		.await?
		.unwrap_or("auto".to_string())
		.to_lowercase();

	println!("Tried source language:   ", attempted_source_language);
	println!("Tried target language:   ", attempted_target_language);
	println!("Current source language: ", current_source_language);
	println!("Current target language: ", current_target_language);

	if attempted_target_language != current_target_language {
		// Include the original language and the translation language and the ones it used.
		return Err(anyhow!("The translation was not successful.\nYou provided `{}` to `{}`.\nThe languages it used were `{}` to `{}`.\nThis usually means the target language is either not supported or was the source language.", attempted_source_language, attempted_target_language, current_source_language, current_target_language));
	}

	Ok(TranslationResult {
		translated_text,
		url,
		source_language: result_source_language,
		target_language: current_target_language,
		auto_detected_source_language: current_source_language == "auto",
	})
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
