use anyhow::Error;

use crate::types::Context;

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

/// Compile and run Rust code in a playground
#[poise::command(
	slash_command,
	owners_only,
	track_edits,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	help_text_fn = "play_help",
	category = "Playground"
)]
pub async fn play(
	ctx: Context<'_>,
	// flags: poise::KeyValueArgs,
	code: String,
) -> Result<(), Error> {
	play_or_eval(
		ctx,
		poise::KeyValueArgs::default(),
		false,
		code,
		ResultHandling::None,
	)
	.await
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

/// Compile and run Rust code with warnings
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

/// Evaluate a single Rust expression
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
