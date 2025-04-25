use std::time::Duration;

use anyhow::{anyhow, Result};
use heck::{ToSnakeCase, ToTitleCase};
use inline_format::format;
use poise::{
	serenity_prelude::{
		ComponentInteractionCollector, ComponentInteractionDataKind,
		CreateActionRow, CreateAttachment, CreateButton, CreateComponent,
		CreateContainer, CreateInteractionResponse,
		CreateInteractionResponseMessage, CreateMediaGallery,
		CreateMediaGalleryItem, CreateSelectMenu, CreateSelectMenuKind,
		CreateSelectMenuOption, CreateTextDisplay, CreateUnfurledMediaItem,
		MessageFlags,
	},
	CreateReply,
};
use secrecy::ExposeSecret;

const ACCENT_COLOR: u32 = 0xff6600;

use crate::config::Context;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PagePosition {
	First,
	Middle,
	Last,
	FirstAndLast,
}

#[derive(Clone, Debug)]
pub struct WolframPage<'a> {
	pub query: String,
	pub components: Vec<CreateComponent<'a>>,
	pub attachments: Vec<CreateAttachment<'a>>,
	pub position: PagePosition,
}
impl<'a> WolframPage<'a> {
	/// 30 - interaction_buttons(&self) - 2 Containers
	pub const MAX_COMPONENTS: usize = 10;
	/// Discord limit.
	pub const MAX_ATTACHMENTS: usize = 10;

	pub fn new(query: impl ToString, position: PagePosition) -> Self {
		Self {
			query: query.to_string(),
			components: Vec::new(),
			attachments: Vec::new(),
			position,
		}
	}

	pub fn interaction_controls(
		&self,
		current_page: usize,
		total_pages: usize,
		expired: bool,
	) -> CreateComponent<'a> {
		CreateComponent::Container(
			CreateContainer::new(
				[
					CreateComponent::ActionRow(CreateActionRow::Buttons(
						[
							CreateButton::new("first").label("<--").disabled(
								self.position == PagePosition::First
									|| self.position
										== PagePosition::FirstAndLast
									|| expired,
							),
							CreateButton::new("prev").label("<-").disabled(
								self.position == PagePosition::First
									|| self.position
										== PagePosition::FirstAndLast
									|| expired,
							),
							CreateButton::new("next").label("->").disabled(
								self.position == PagePosition::Last
									|| self.position
										== PagePosition::FirstAndLast
									|| expired,
							),
							CreateButton::new("last").label("-->").disabled(
								self.position == PagePosition::Last
									|| self.position
										== PagePosition::FirstAndLast
									|| expired,
							),
							CreateButton::new_link(format!(
								"https://www.wolframalpha.com/input?i=",
								self.query
							))
							.label("View Online"),
						]
						.to_vec()
						.into(),
					)),
					CreateComponent::ActionRow(CreateActionRow::SelectMenu(
						CreateSelectMenu::new(
							"jump",
							CreateSelectMenuKind::String {
								options: (1..=total_pages)
									.into_iter()
									.map(|page_number| {
										CreateSelectMenuOption::new(
											format!("Page ", page_number),
											page_number.to_string(),
										)
									})
									.collect(),
							},
						)
						.placeholder(format!("Page ", current_page))
						.min_values(1)
						.max_values(1)
						.disabled(expired),
					)),
				]
				.to_vec()
				.to_owned(),
			)
			.accent_color(ACCENT_COLOR),
		)
	}

	pub fn message_components(
		&self,
		current_page: usize,
		total_pages: usize,
		expired: bool,
	) -> Vec<CreateComponent<'a>> {
		[
			CreateComponent::Container(
				CreateContainer::new(self.components.clone())
					.accent_color(ACCENT_COLOR),
			),
			self.interaction_controls(current_page, total_pages, expired),
		]
		.to_vec()
		.to_owned()
	}

	pub fn to_response(
		&self,
		current_page: usize,
		total_pages: usize,
		expired: bool,
	) -> CreateInteractionResponseMessage<'a> {
		CreateInteractionResponseMessage::new()
			.flags(MessageFlags::IS_COMPONENTS_V2)
			.components(self.message_components(
				current_page,
				total_pages,
				expired,
			))
			.add_files(self.attachments.to_owned())
	}

	pub fn to_reply(
		&self,
		current_page: usize,
		total_pages: usize,
		expired: bool,
	) -> CreateReply<'a> {
		let mut reply = CreateReply::new()
			.flags(MessageFlags::IS_COMPONENTS_V2)
			.components(self.message_components(
				current_page,
				total_pages,
				expired,
			));
		for attachment in self.attachments.iter() {
			reply = reply.attachment(attachment.to_owned());
		}
		reply
	}
}

pub fn generate_timeouts(time: Duration) -> String {
	format!(
		"&scantimeout=",
		time.as_secs(),
		"&podtimeout=",
		time.as_secs(),
		"&formattimeout=",
		time.as_secs(),
		"&parsetimeout=",
		time.as_secs(),
		"&totaltimeout=",
		time.as_secs() * 4,
	)
}

#[derive(Clone, Debug)]
pub struct WolframPod<'a> {
	pub title: String,
	pub description: Option<String>,
	pub attachment: Option<CreateAttachment<'a>>,
}
impl<'a> WolframPod<'a> {
	pub fn new(
		title: String,
		description: Option<String>,
		attachment: Option<CreateAttachment<'a>>,
	) -> Self {
		Self {
			title,
			description,
			attachment,
		}
	}

	pub fn message_components(&mut self) -> Vec<CreateComponent<'a>> {
		let content = match self.description.clone() {
			Some(description) => {
				format!("## ", self.title, "\n```rs\n", description, "\n```")
			}
			None => format!("## ", self.title),
		};
		let mut components = [CreateComponent::TextDisplay(
			CreateTextDisplay::new(content),
		)]
		.to_vec()
		.to_owned();
		if let Some(attachment) = &self.attachment {
			components.push(CreateComponent::MediaGallery(
				CreateMediaGallery::new(
					[CreateMediaGalleryItem::new(
						CreateUnfurledMediaItem::new(format!(
							"attachment://",
							attachment.filename
						)),
					)]
					.to_vec()
					.to_owned(),
				),
			));
		}

		components
	}
}
pub trait VecWolframPod<'a> {
	fn get_by_title(&mut self, name: &str) -> Option<&mut WolframPod<'a>>;
}
impl<'a> VecWolframPod<'a> for Vec<WolframPod<'a>> {
	fn get_by_title(&mut self, name: &str) -> Option<&mut WolframPod<'a>> {
		self.iter_mut()
			.find(|pod| pod.title.to_snake_case() == name.to_snake_case())
	}
}

/// Asks Wolfram Alpha a question.
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn wolfram(
	ctx: Context<'_>,
	#[description = "The natural language to query Wolfram Alpha with."]
	query: String,
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

	let wolfram_alpha_full_app_id = if let Some(app_id) =
		ctx.data().config.wolfram_alpha_full_app_id.clone()
	{
		app_id
	} else {
		return Err(anyhow!("wolfram_alpha_full_app_id is not set."));
	};

	let query = urlencoding::encode(&query);

	let full_results_api_url = format!(
		"https://api.wolframalpha.com/v2/query?input=",
		query,
		generate_timeouts(Duration::from_secs(60)),
		"&format=plaintext,image&output=json&async=false&units=metric&appid=",
		wolfram_alpha_full_app_id.expose_secret()
	);

	let full_response =
		ctx.data().http.get(full_results_api_url).send().await?;

	if !full_response.status().is_success() {
		return Err(anyhow!(
			"Failed to get response from Wolfram Alpha Full Results API."
		));
	}

	let json: serde_json::Value = full_response.json().await?;

	let numpods = json["queryresult"]["numpods"]
		.as_u64()
		.ok_or_else(|| anyhow!("Invalid numpods format"))?;

	// let didyoumeans = json["queryresult"]["didyoumeans"].as_array();

	// if let Some(didyoumeans) = didyoumeans {
	// 	let mut embed = CreateEmbed::new()
	// 		.title("Did you mean...")
	// 		.color(EMBED_COLOR);

	// 	for didyoumean in didyoumeans {
	// 		let val = didyoumean["val"].as_str().unwrap_or("???");
	// 		let score = didyoumean["score"]
	// 			.as_str()
	// 			.unwrap_or("0.0")
	// 			.parse::<f64>()?;
	// 		embed = embed.field(
	// 			format!((score * 100.0).round(), "% Confidence"),
	// 			format!("```rust\n", val, "```"),
	// 			true,
	// 		);
	// 	}

	// 	reply = reply.embed(embed);
	// }

	if numpods == 0 {
		let reply = CreateReply::new()
			.flags(MessageFlags::IS_COMPONENTS_V2)
			.components(
				[CreateComponent::Container(CreateContainer::new(
					[CreateComponent::ActionRow(CreateActionRow::Buttons(
						[CreateButton::new_link(format!(
							"https://www.wolframalpha.com/input?i=",
							query
						))
						.label("View Online")]
						.to_vec()
						.into(),
					))]
					.to_vec()
					.to_owned(),
				))]
				.to_vec()
				.to_owned(),
			)
			.ephemeral(ephemeral);

		ctx.send(reply.clone()).await?;

		return Ok(());
	}

	let json_pods = json["queryresult"]["pods"]
		.as_array()
		.ok_or_else(|| anyhow!("Invalid response format"))?;

	let mut pods: Vec<WolframPod> = Vec::new();

	for json_pod in json_pods {
		let title = json_pod["title"]
			.as_str()
			.unwrap_or("Untitled")
			.to_title_case();
		let subpods = json_pod["subpods"]
			.as_array()
			.ok_or_else(|| anyhow!("Invalid subpods format"))?;

		for (i, subpod) in subpods.iter().enumerate() {
			let subpod_title = if subpods.len() > 1 {
				format!(title, " ", i + 1)
			} else {
				title.to_string()
			}
			.to_title_case();
			let mut subpod_description = None;
			let mut subpod_attachment = None;

			let plaintext = subpod["plaintext"].as_str();
			if let Some(plaintext) = plaintext {
				if !plaintext.is_empty() {
					subpod_description = Some(plaintext.to_string());
				}
			}
			let image_url = subpod["img"]["src"].as_str();
			if let Some(image_url) = image_url {
				// Download the image so that it doesn't expire and supports
				// alt text for whenever Discord remembers to add it to embeds.
				let image_name =
					format!(title.to_snake_case(), "_", i + 1, ".webp");
				let mut image_attachment = CreateAttachment::bytes(
					ctx.data()
						.http
						.get(image_url)
						.send()
						.await?
						.bytes()
						.await?,
					image_name.clone(),
				);
				if subpod_description.is_some() {
					if let Some(plaintext) = plaintext {
						image_attachment =
							image_attachment.description(plaintext);
					}
				}
				subpod_attachment = Some(image_attachment);
			}

			if subpod_description.is_none() && subpod_attachment.is_none() {
				subpod_description = Some("Pod didn't have any parsable content.\nView it online to see the full result.".to_string());
			}

			pods.push(WolframPod::new(
				subpod_title,
				subpod_description,
				subpod_attachment,
			));
		}
	}

	let mut pages: Vec<WolframPage> = Vec::new();
	let mut first_page = WolframPage::new(query.clone(), PagePosition::First);

	// Remove the "Input" pod from the map.
	// The user already knows that information.
	pods.retain(|pod| pod.title.to_title_case() != "Input");

	// Add a default result.
	let mut added_component = false;
	if let Some(pod) = pods.get_by_title("Input Interpretation") {
		first_page.components.append(&mut pod.message_components());

		if let Some(attachment) = pod.attachment.clone() {
			first_page.attachments.push(attachment);
		}

		pods.retain(|pod| {
			pod.title.to_title_case() != "Input Interpretation"
		});
		added_component = true;
	}
	if let Some(pod) = pods.get_by_title("Result") {
		first_page.components.append(&mut pod.message_components());

		if let Some(attachment) = pod.attachment.clone() {
			first_page.attachments.push(attachment);
		}

		pods.retain(|pod| pod.title.to_title_case() != "Result");
		added_component = true;
	}
	// As a backup, add the first one.
	if !added_component {
		// If the default embed wasn't added, add the first one.
		if let Some(pod) = pods.iter_mut().next() {
			first_page.components.append(&mut pod.message_components());

			if let Some(attachment) = pod.attachment.clone() {
				first_page.attachments.push(attachment);
			}

			// Remove the pod.
			pods.remove(0);
		}
	}
	pages.push(first_page.clone());

	// Create the rest of the pages.
	let mut current_page =
		WolframPage::new(query.clone(), PagePosition::Middle);
	let pods_len = pods.len();
	for (i, pod) in pods.iter_mut().enumerate() {
		// Very important.
		// Change this when you add more components to the page.
		if current_page.components.len() + 2 > WolframPage::MAX_COMPONENTS {
			pages.push(current_page);
			current_page =
				WolframPage::new(query.clone(), PagePosition::Middle);
		}

		current_page
			.components
			.append(&mut pod.message_components());

		if let Some(attachment) = pod.attachment.clone() {
			current_page.attachments.push(attachment);
		}

		if i == pods_len - 1 {
			current_page.position = PagePosition::Last;
		}
	}
	if current_page.components.len() > 0 {
		pages.push(current_page);
	}

	let reply = first_page
		.to_reply(1, pages.len(), false)
		.ephemeral(ephemeral);
	let message = ctx.send(reply.clone()).await?;

	let mut current_page_index = 0;

	while match ComponentInteractionCollector::new(&ctx.serenity_context())
		.message_id(message.message().await?.id)
		.timeout(Duration::from_secs(60 * 5))
		.await
	{
		Some(ref interaction) => {
			let mut changed = false;
			match interaction.data.custom_id.as_str() {
				"first" => {
					current_page_index = 0;
					changed = true;
				}
				"next" => {
					current_page_index += 1;
					changed = true;
				}
				"prev" => {
					current_page_index -= 1;
					changed = true;
				}
				"last" => {
					current_page_index = pages.len() - 1;
					changed = true;
				}
				"jump" => match interaction.data.kind.clone() {
					ComponentInteractionDataKind::StringSelect { values } => {
						let last = current_page_index;
						current_page_index = values
							.get(0)
							.unwrap_or(&"1".to_string())
							.parse::<usize>()
							.unwrap_or(1) - 1;
						changed = last != current_page_index;
					}
					_ => {}
				},
				_ => {}
			}

			if changed {
				interaction
					.create_response(
						&ctx.http(),
						CreateInteractionResponse::UpdateMessage(
							pages
								.get(current_page_index)
								.unwrap()
								.to_response(
									current_page_index + 1,
									pages.len(),
									false,
								),
						),
					)
					.await?;
			} else {
				interaction
					.create_response(
						&ctx.http(),
						CreateInteractionResponse::Acknowledge,
					)
					.await?;
			}

			true
		}
		_ => false,
	} {}

	message
		.edit(
			ctx,
			pages.get(current_page_index).unwrap().to_reply(
				current_page_index + 1,
				pages.len(),
				true,
			),
		)
		.await?;

	Ok(())
}
