use syn::{Ident, Expr, Token};
use syn::parse::{ParseStream, Result};
use syn_derive::{Parse, ToTokens};

fn check_stream_prefix (input: ParseStream <'_>) -> Result <()>
{
	input . parse::<Ident> ()?;
	input . parse::<Token! [->]> ()?;

	Ok (())
}

fn check_future_prefix (input: ParseStream <'_>) -> Result <()>
{
	input . parse::<IdentOrUnderscore> ()?;
	input . parse::<Token! [=]> ()?;

	Ok (())
}

#[derive (Parse)]
pub enum EventPattern
{
	#[parse (peek_func = |input| check_stream_prefix (input) . is_ok ())]
	Stream (StreamEventPattern),
	#[parse (peek_func = |input| check_future_prefix (input) . is_ok ())]
	Future (FutureEventPattern)
}

#[derive (Parse, ToTokens)]
pub enum IdentOrUnderscore
{
	#[parse (peek = Token! [_])]
	Underscore (Token! [_]),
	Ident (Ident)
}

#[allow (dead_code)]
#[derive (Parse)]
pub struct StreamEventPattern
{
	pub stream: Ident,
	pub r_arrow_token: Token! [->],
	pub item: IdentOrUnderscore,
	pub question_token: Option <Token! [?]>,
	pub fat_arrow_token: Token! [=>],
	pub handler: Expr
}

#[allow (dead_code)]
#[derive (Parse)]
pub struct FutureEventPattern
{
	pub value: IdentOrUnderscore,
	pub eq_token: Token! [=],
	pub future: Expr,
	pub fat_arrow_token: Token! [=>],
	pub handler: Expr
}
