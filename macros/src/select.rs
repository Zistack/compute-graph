use syn::Token;
use syn::punctuated::Punctuated;
use syn::parse::{Parser, Result, Error};
use quote::quote;

use crate::event_pattern::*;

fn implement_stream_pattern (stream_pattern: StreamEventPattern)
-> Result <proc_macro2::TokenStream>
{
	let StreamEventPattern {stream, item, question_token, handler, ..} =
		stream_pattern;

	if question_token . is_some ()
	{
		return Err
		(
			Error::new_spanned
			(
				question_token,
				"fallible streams are not supported by this macro; use select_fallible instead"
			)
		);
	}

	let tokens = quote!
	{
		#item = #stream . next () => match #item
		{
			std::option::Option::Some (#item) =>
				std::convert::Into::<compute_graph::exit_status::ShouldTerminateClean>::into (#handler),
			std::option::Option::None =>
				compute_graph::exit_status::ShouldTerminateClean::new ((), false)
		}
	};

	Ok (tokens)
}

fn implement_future_pattern (future_pattern: FutureEventPattern)
-> proc_macro2::TokenStream
{
	let FutureEventPattern {value, future, handler, ..} = future_pattern;

	quote!
	{
		#value = #future =>
			std::convert::Into::<compute_graph::exit_status::ShouldTerminateClean>::into (#handler)
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

fn select_inner (event_patterns: Punctuated <EventPattern, Token! [,]>)
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
		{
			let term_status: compute_graph::exit_status::ShouldTerminateClean =
				tokio::select!
				(
					biased;
					#(#implemented_event_patterns),*
				);

			term_status
		}
	};

	Ok (tokens)
}

fn try_select_impl (input: proc_macro::TokenStream)
-> Result <proc_macro2::TokenStream>
{
	let event_patterns = Punctuated::parse_terminated . parse (input)?;

	select_inner (event_patterns)
}

pub fn select_impl (input: proc_macro::TokenStream) -> proc_macro::TokenStream
{
	try_select_impl (input)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
