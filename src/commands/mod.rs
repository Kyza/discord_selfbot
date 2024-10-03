macro_rules! slash_commands {
	($name:ident) => {
		mod $name;
		pub use $name::*;
	};
	($($name:ident),* $(,)?) => {
		$(
			slash_commands!($name);
		)*
	};
}

slash_commands![age, github, cobalt, crates, utilities, timestamp];
