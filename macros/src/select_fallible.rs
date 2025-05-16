use syn::Token;
use syn::punctuated::Punctuated;
use syn::parse::{Parser, Result, Error};
use quote::quote;

use crate::event_pattern::*;

fn implement_stream_pattern (stream_pattern: StreamEventPattern)
-> proc_macro2::TokenStream
{
	let StreamEventPattern {stream, item, question_token, handler, ..}
		= stream_pattern;

	let stream_close_status = if question_token . is_some ()
	{
		quote! (compute_graph::exit_status::ExitStatus::Spurious)
	}
	else
	{
		quote! (compute_graph::exit_status::ExitStatus::Clean)
	};

	quote!
	{
		#item = #stream . next () => match #item
		{
			std::option::Option::Some (#item) =>
				std::convert::Into::<compute_graph::exit_status::ShouldTerminateWithStatus>::into (#handler),
			std::option::Option::None =>
				compute_graph::exit_status::ShouldTerminateWithStatus::new
				(
					(),
					std::option::Option::Some (#stream_close_status)
				)
		}
	}
}

fn implement_future_pattern (future_pattern: FutureEventPattern)
-> proc_macro2::TokenStream
{
	let FutureEventPattern {value, future, handler, ..} = future_pattern;

	quote!
	{
		#value = #future =>
			std::convert::Into::<compute_graph::exit_status::ShouldTerminateWithStatus>::into (#handler)
	}
}

fn implement_event_pattern (event_pattern: EventPattern)
-> proc_macro2::TokenStream
{
	match event_pattern
	{
		EventPattern::Stream (stream_pattern) =>
			implement_stream_pattern (stream_pattern),
		EventPattern::Future (future_pattern) =>
			implement_future_pattern (future_pattern)
	}
}

fn select_fallible_inner (event_patterns: Punctuated <EventPattern, Token! [,]>)
-> proc_macro2::TokenStream
{
	let implemented_event_patterns = event_patterns
		. into_iter ()
		. map (|event_pattern| implement_event_pattern (event_pattern));

	quote!
	{
		{
			let term_status: compute_graph::exit_status::ShouldTerminateWithStatus =
				tokio::select!
				(
					#(#implemented_event_patterns),*
				);

			term_status
		}
	}
}

fn try_select_fallible_impl (input: proc_macro::TokenStream)
-> Result <proc_macro2::TokenStream>
{
	let event_patterns = Punctuated::parse_terminated . parse (input)?;

	Ok (select_fallible_inner (event_patterns))
}

pub fn select_fallible_impl (input: proc_macro::TokenStream)
-> proc_macro::TokenStream
{
	try_select_fallible_impl (input)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
