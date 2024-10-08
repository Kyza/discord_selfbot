use anyhow::Result;
use caith::{RollResultType, Roller};
use poise::{
	serenity_prelude::{CreateAllowedMentions, CreateAttachment},
	CreateReply,
};

use crate::types::Context;

/// Rolls dice.
///
/// xdy [OPTIONS] [TARGET] [FAILURE] [! REASON]
/// roll `x` dice(s) with `y` sides
/// `y` can also be "F" or "f" for fudge dice. In this case, no option applies and ignored if provided.
/// Options:
/// + - / * : modifiers
/// e# : Explode value. If number is omitted, we use dice sides
/// ie# or !# : Indefinite explode value, If number is omitted, we use dice sides
/// K#  : Keeping # highest (upperacse "K")
/// k#  : Keeping # lowest (lowercase "k")
/// D#  : Dropping the highest (uppercase "D")
/// d#  : Dropping the lowest (lowercase "d")
/// r#  : Reroll if <= value
/// ir# : Indefinite reroll if <= value

/// Target:
/// t#  : minimum value to count as success
/// tt# : minimum value to count as two successes
/// t[<list of numbers>] : enumeration of values considered as success

/// Failure:
/// f# : value under which it's counted as failure

/// Repetition:
/// a roll can be repeated with `^` operator: `(2d6 + 6) ^ 8` will roll eight times the expression.

/// Summed repetition:
/// with the `^+` operator, the roll will be repeated and all the totals summed.

/// Sorted repetition:
/// with the `^#` operator, the roll will be repeated and sorted by total.

/// Reason:
/// : : Any text after `:` will be a comment
#[poise::command(
	owners_only,
	track_edits,
	slash_command,
	install_context = "User",
	interaction_context = "Guild|BotDm|PrivateChannel",
	ephemeral
)]
pub async fn roll(
	ctx: Context<'_>,
	#[description = "The expression to roll."] text: String,
	#[description = "Whether or not to show the message."] ephemeral: Option<
		bool,
	>,
) -> Result<()> {
	let roll_result = Roller::new(&text)?.roll()?;
	let roll_result = roll_result.get_result();
	let text_markdown = format_roll(&roll_result, true);

	let mut reply = CreateReply::default()
		.allowed_mentions(CreateAllowedMentions::default())
		.ephemeral(ephemeral.unwrap_or(true));

	reply = if text_markdown.len() > 2000 {
		let text_plain = format_roll(&roll_result, false);
		reply.attachment(CreateAttachment::bytes(
			text_plain.as_bytes().to_vec(),
			"roll_result.md".to_string(),
		))
	} else {
		reply.content(text_markdown)
	};

	ctx.send(reply).await?;
	Ok(())
}

fn format_roll(roll: &RollResultType, md: bool) -> String {
	match roll {
		RollResultType::Single(result) => result.to_string(md),
		RollResultType::Repeated(result) => result
			.iter()
			.map(|r| r.to_string(md))
			.collect::<Vec<String>>()
			.join("\n"),
	}
}
