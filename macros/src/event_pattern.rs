use syn::{Ident, Block, Expr, Token};
use syn::parse::{ParseStream, Result};
use syn_derive::{Parse, ToTokens};

mod kw
{
	syn::custom_keyword! (then);
}

#[derive (Parse, ToTokens)]
pub enum IdentOrUnderscore
{
	#[parse (peek = Token! [_])]
	Underscore (Token! [_]),
	Ident (Ident)
}

fn check_shutdown_prefix (input: ParseStream <'_>) -> Result <()>
{
	input . parse::<Token! [?]> ()?;

	Ok (())
}

fn check_stream_prefix (input: ParseStream <'_>) -> Result <()>
{
	input . parse::<Ident> ()?;
	input . parse::<Option <Token! [?]>> ()?;
	input . parse::<Token! [->]> ()?;
	input . parse::<IdentOrUnderscore> ()?;
	input . parse::<Token! [=>]> ()?;

	Ok (())
}

fn check_stream_iter_prefix (input: ParseStream <'_>) -> Result <()>
{
	input . parse::<Ident> ()?;
	input . parse::<Option <Token! [?]>> ()?;
	input . parse::<Token! [->]> ()?;
	input . parse::<IdentOrUnderscore> ()?;
	input . parse::<Token! [..]> ()?;

	Ok (())
}

fn check_future_prefix (input: ParseStream <'_>) -> Result <()>
{
	input . parse::<IdentOrUnderscore> ()?;
	input . parse::<Token! [=]> ()?;

	Ok (())
}

#[allow (dead_code)]
#[derive (Parse)]
pub struct ShutdownEventPattern
{
	pub question_token: Token! [?],
	pub shutdown: Expr
}

#[allow (dead_code)]
#[derive (Parse)]
pub struct StreamEventPattern
{
	pub stream: Ident,
	pub question_token: Option <Token! [?]>,
	pub r_arrow_token: Token! [->],
	pub item: IdentOrUnderscore,
	pub fat_arrow_token: Token! [=>],
	pub handler: Expr
}

#[allow (dead_code)]
#[derive (Parse)]
pub struct FinishHandler
{
	pub with_token: kw::then,
	pub handler: Expr
}

fn parse_maybe_finish_handler (input: ParseStream <'_>)
-> Result <Option <FinishHandler>>
{
	if input . peek (kw::then)
	{
		Ok (Some (input . parse ()?))
	}
	else
	{
		Ok (None)
	}
}

#[allow (dead_code)]
#[derive (Parse)]
pub struct StreamIterEventPattern
{
	pub stream: Ident,
	pub question_token: Option <Token! [?]>,
	pub r_arrow_token: Token! [->],
	pub item: IdentOrUnderscore,
	pub dot_dot_token: Token! [..],
	pub fat_arrow_token: Token! [=>],
	pub item_handler: Block,
	#[parse (parse_maybe_finish_handler)]
	pub finish_handler: Option <FinishHandler>
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

#[derive (Parse)]
pub enum EventPattern
{
	#[parse (peek_func = |input| check_shutdown_prefix (input) . is_ok ())]
	Shutdown (ShutdownEventPattern),
	#[parse (peek_func = |input| check_stream_prefix (input) . is_ok ())]
	Stream (StreamEventPattern),
	#[parse (peek_func = |input| check_stream_iter_prefix (input) . is_ok ())]
	StreamIter (StreamIterEventPattern),
	#[parse (peek_func = |input| check_future_prefix (input) . is_ok ())]
	Future (FutureEventPattern)
}
