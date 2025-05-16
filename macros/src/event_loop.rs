use syn::{Ident, Expr, Token};
use syn::punctuated::Punctuated;
use syn::parse::{Parser, Result, Error};
use quote::quote;

use crate::event_pattern::*;

fn implement_handler (handler: Expr) -> proc_macro2::TokenStream
{
	quote!
	{
		{
			let term_status: compute_graph::exit_status::ShouldTerminateClean =
				std::convert::Into::into (#handler);

			if term_status . should_terminate
			{
				break compute_graph::exit_status::AlwaysClean::new (());
			}
		}
	}
}

fn implement_stream_pattern (stream_pattern: StreamEventPattern)
-> Result <proc_macro2::TokenStream>
{
	let StreamEventPattern {stream, item, question_token, handler, ..} =
		stream_pattern;

	let implemented_handler = implement_handler (handler);

	if question_token . is_some ()
	{
		return Err
		(
			Error::new_spanned
			(
				question_token,
				"fallible streams are not supported by this macro; use event_loop_fallible instead"
			)
		);
	}

	let intermediate = Ident::new ("__item", proc_macro2::Span::mixed_site ());

	let tokens = quote!
	{
		#intermediate = #stream . next () => match #intermediate
		{
			std::option::Option::Some (#item) => #implemented_handler,
			std::option::Option::None =>
				break compute_graph::exit_status::AlwaysClean::new (())
		}
	};

	Ok (tokens)
}

fn implement_future_pattern (future_pattern: FutureEventPattern)
-> proc_macro2::TokenStream
{
	let FutureEventPattern {value, future, handler, ..} = future_pattern;

	let implemented_handler = implement_handler (handler);

	quote!
	{
		#value = #future => #implemented_handler
	}
}

fn implement_event_pattern (event_pattern: EventPattern)
-> Result <proc_macro2::TokenStream>
{
	match event_pattern
	{
		EventPattern::Stream (stream_pattern) =>
			implement_stream_pattern (stream_pattern),
		EventPattern::Future (future_pattern) =>
			Ok (implement_future_pattern (future_pattern))
	}
}

// Specializing for a single handler would be cool.
fn event_loop_inner (event_patterns: Punctuated <EventPattern, Token! [,]>)
-> Result <proc_macro2::TokenStream>
{
	let mut implemented_event_patterns =
		Vec::with_capacity (event_patterns . len ());

	for event_pattern in event_patterns
	{
		implemented_event_patterns
			. push (implement_event_pattern (event_pattern)?);
	}

	let tokens = quote!
	{
		loop
		{
			tokio::select!
			(
				biased;
				#(#implemented_event_patterns),*
			);
		}
	};

	Ok (tokens)
}

fn try_event_loop_impl (input: proc_macro::TokenStream)
-> Result <proc_macro2::TokenStream>
{
	let event_patterns = Punctuated::parse_terminated . parse (input)?;

	event_loop_inner (event_patterns)
}

pub fn event_loop_impl (input: proc_macro::TokenStream)
-> proc_macro::TokenStream
{
	try_event_loop_impl (input)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
