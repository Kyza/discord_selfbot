use anyhow::{Error, Result};
use poise::{
	execute_modal, serenity_prelude::Message, ApplicationContext, Modal,
};

use crate::{
	commands::playground::util::extract_codeblock,
	types::{Context, Data},
};

use super::{
	api::{CrateType, PlayResult, PlaygroundRequest},
	util::{
		edit_reply, format_play_eval_stderr, generic_help, maybe_wrapped,
		parse_flags, stub_message, GenericHelp, ResultHandling,
	},
};

// play and eval work similarly, so this function abstracts over the two
async fn play_or_eval(
	ctx: Context<'_>,
	flags: poise::KeyValueArgs,
	force_warnings: bool, // If true, force enable warnings regardless of flags
	code: String,
	result_handling: ResultHandling,
) -> Result<(), Error> {
	let message = ctx.say(stub_message(ctx)).await?;

	let code = maybe_wrapped(
		&code,
		result_handling,
		ctx.prefix().contains("Sweat"),
		ctx.prefix().contains("OwO") || ctx.prefix().contains("Cat"),
	);
	let (mut flags, flag_parse_errors) = parse_flags(flags);

	if force_warnings {
		flags.warn = true;
	}

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

/// Compile and run Rust code in a playground.
#[poise::command(
	context_menu_command = "Rust Playground",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	help_text_fn = "play_help",
	category = "Playground"
)]
pub async fn rust_playground_context_menu(
	ctx: Context<'_>,
	#[description = "Message with Rust code to play as link or ID."]
	message: Message,
) -> Result<(), Error> {
	play_or_eval(
		ctx,
		poise::KeyValueArgs::default(),
		false,
		extract_codeblock(message.content).expect("Has no codeblock."),
		ResultHandling::None,
	)
	.await
}

#[derive(Debug, Modal)]
#[allow(dead_code)] // fields only used for Debug print
struct PlaygroundModal {
	first_input: String,
	second_input: Option<String>,
}

/// Compile and run Rust code in a playground.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	help_text_fn = "play_help",
	category = "Playground"
)]
pub async fn rust_playground(
	ctx: ApplicationContext<'_, Data, Error>,
) -> Result<()> {
	let data = PlaygroundModal::execute(ctx).await?;

	ctx.reply(format!("{:?}", data)).await?;

	Ok(())

	// play_or_eval(
	// 	ctx,
	// 	poise::KeyValueArgs::default(),
	// 	false,
	// 	code,
	// 	ResultHandling::None,
	// )
	// .await
}

#[must_use]
pub fn play_help() -> String {
	generic_help(GenericHelp {
		command: "play",
		desc: "Compile and run Rust code",
		mode_and_channel: true,
		warn: true,
		run: false,
		example_code: "code",
	})
}

/// Compile and run Rust code with warnings.
#[poise::command(
	context_menu_command = "Rust Warnings Playground",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
   hide_in_help, // don't clutter help menu with something that ?play can do too
   help_text_fn = "playwarn_help",
   category = "Playground"
)]
pub async fn playwarn_context_menu(
	ctx: Context<'_>,
	#[description = "Message with Rust code to play as link or ID."]
	message: Message,
) -> Result<(), Error> {
	play_or_eval(
		ctx,
		poise::KeyValueArgs::default(),
		true,
		extract_codeblock(message.content).expect("Has no codeblock."),
		ResultHandling::None,
	)
	.await
}

/// Compile and run Rust code with warnings.
#[poise::command(
   slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
   hide_in_help, // don't clutter help menu with something that ?play can do too
   help_text_fn = "playwarn_help",
   category = "Playground"
)]
pub async fn playwarn(
	ctx: Context<'_>,
	// flags: poise::KeyValueArgs,
	code: String,
) -> Result<(), Error> {
	play_or_eval(
		ctx,
		poise::KeyValueArgs::default(),
		true,
		code,
		ResultHandling::None,
	)
	.await
}

#[must_use]
pub fn playwarn_help() -> String {
	generic_help(GenericHelp {
		command: "playwarn",
		desc: "Compile and run Rust code with warnings. Equivalent to \
		       `?play warn=true`",
		mode_and_channel: true,
		warn: false,
		run: false,
		example_code: "code",
	})
}

/// Evaluate a single Rust expression.
#[poise::command(
	context_menu_command = "Rust Eval Playground",
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	help_text_fn = "eval_help",
	category = "Playground"
)]
pub async fn eval_context_menu(
	ctx: Context<'_>,
	#[description = "Message with Rust code to play as link or ID."]
	message: Message,
) -> Result<(), Error> {
	play_or_eval(
		ctx,
		poise::KeyValueArgs::default(),
		false,
		extract_codeblock(message.content).expect("Has no codeblock."),
		ResultHandling::Print,
	)
	.await
}

/// Evaluate a single Rust expression.
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	help_text_fn = "eval_help",
	category = "Playground"
)]
pub async fn eval(
	ctx: Context<'_>,
	// flags: poise::KeyValueArgs,
	code: String,
) -> Result<(), Error> {
	play_or_eval(
		ctx,
		poise::KeyValueArgs::default(),
		false,
		code,
		ResultHandling::Print,
	)
	.await
}

#[must_use]
pub fn eval_help() -> String {
	generic_help(GenericHelp {
		command: "eval",
		desc: "Compile and run Rust code",
		mode_and_channel: true,
		warn: true,
		run: false,
		example_code: "code",
	})
}
