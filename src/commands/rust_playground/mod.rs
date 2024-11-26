
use anyhow::{anyhow, Result};
use api::{CrateType, PlayResult, PlaygroundRequest};
use poise::{
	serenity_prelude::Message,
	CreateReply, Modal,
};
use util::{
	edit_reply, format_play_eval_stderr, get_codeblocks, maybe_wrapped,
	parse_flags, stub_message, ResultHandling,
};

use crate::{
	key_value_args_utils,
	types::{ApplicationContext, Context},
};

mod api;
mod util;

// play and eval work similarly, so this function abstracts over the two
async fn run_playground(
	ctx: Context<'_>,
	flags: poise::KeyValueArgs,
	code: String,
	result_handling: ResultHandling,
	ephemeral: bool,
) -> Result<()> {
	let message = ctx
		.send(
			CreateReply::default()
				.content(stub_message(ctx))
				.ephemeral(ephemeral),
		)
		.await?;

	let code = maybe_wrapped(&code, result_handling, false, true);

	let (flags, flag_parse_errors) = parse_flags(flags);

	let mut result: PlayResult = ctx
		.data()
		.http
		.post("https://play.rust-lang.org/execute")
		.json(&PlaygroundRequest {
			code: &code,
			channel: flags.channel,
			crate_type: CrateType::Binary,
			edition: flags.edition,
			mode: flags.mode,
			tests: false,
		})
		.send()
		.await?
		.json()
		.await?;

	result.stderr = format_play_eval_stderr(&result.stderr, flags.warn);

	edit_reply(ctx, message, result, &code, &flags, &flag_parse_errors).await
}

#[derive(Debug, Modal)]
struct PlaygroundModal {
	#[name = "Codeblock Index"]
	#[placeholder = "The 0-based index of the codeblock to run. If not specified, the first codeblock will be run."]
	codeblock_index: Option<String>,
	#[name = "REPL Mode"]
	#[placeholder = "Whether or not to print the last expression."]
	repl: Option<String>,
	#[placeholder = "The playground flags."]
	flags: Option<String>,
	#[placeholder = "Whether or not to show the message."]
	ephemeral: Option<String>,
}

/// Compile and run Rust code in a playground.
#[poise::command(
	slash_command,
	context_menu_command = "Rust Playground",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	category = "Rust",
	ephemeral
)]
pub async fn rust_playground(
	ctx: ApplicationContext<'_>,
	message: Message,
) -> Result<()> {
	let data = PlaygroundModal::execute(ctx)
		.await?
		.ok_or_else(|| anyhow!("No modal data."))?;

	let ephemeral = match data.ephemeral.as_deref() {
		Some("false") => false,
		Some(_) => true,
		None => false,
	};

	let (_, flags) =
		key_value_args_utils::pop_from(&data.flags.unwrap_or(String::new()));

	let codeblocks = get_codeblocks(message.content);
	let codeblock_index = data
		.codeblock_index
		.unwrap_or("0".to_string())
		.parse::<usize>()
		.map_err(|e| anyhow!("Invalid codeblock index: {e}"))?;
	let codeblock = codeblocks.get(codeblock_index).ok_or_else(|| {
		anyhow!(
			"Codeblock index out of range: {codeblock_index} of length {}",
			codeblocks.len()
		)
	})?;

	run_playground(
		Context::Application(ctx),
		flags,
		codeblock.to_string(),
		match data.repl.as_deref() {
			Some("false") => ResultHandling::None,
			Some(_) => ResultHandling::Print,
			None => ResultHandling::None,
		},
		ephemeral,
	)
	.await
}
