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
			let term_status: compute_graph::exit_status::ShouldTerminateWithStatus =
				std::convert::Into::into (#handler);

			if let std::option::Option::Some (exit_status) =
				term_status . should_terminate_status
			{
				break compute_graph::exit_status::WithStatus::new ((), exit_status)
			}
		}
	}
}

fn implement_stream_pattern (stream_pattern: StreamEventPattern)
-> proc_macro2::TokenStream
{
	let StreamEventPattern {stream, item, question_token, handler, ..} =
		stream_pattern;

	let implemented_handler = implement_handler (handler);

	let stream_close_status = if question_token . is_some ()
	{
		quote! (compute_graph::exit_status::ExitStatus::Spurious)
	}
	else
	{
		quote! (compute_graph::exit_status::ExitStatus::Clean)
	};

	let intermediate = Ident::new ("__item", proc_macro2::Span::mixed_site ());

	quote!
	{
		#intermediate = #stream . next () => match #intermediate
		{
			std::option::Option::Some (#item) => #implemented_handler,
			std::option::Option::None =>
				break compute_graph::exit_status::WithStatus::new
				(
					(),
					#stream_close_status
				)
		}
	}
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

fn event_loop_fallible_inner
(
	event_patterns: Punctuated <EventPattern, Token! [,]>
)
-> proc_macro2::TokenStream
{
	let implemented_event_patterns = event_patterns
		. into_iter ()
		. map (|event_pattern| implement_event_pattern (event_pattern));

	quote!
	{
		loop
		{
			tokio::select!
			(
				biased;
				#(#implemented_event_patterns),*
			);
		}
	}
}

fn try_event_loop_fallible_impl (input: proc_macro::TokenStream)
-> Result <proc_macro2::TokenStream>
{
	let event_patterns = Punctuated::parse_terminated . parse (input)?;

	Ok (event_loop_fallible_inner (event_patterns))
}

pub fn event_loop_fallible_impl (input: proc_macro::TokenStream)
-> proc_macro::TokenStream
{
	try_event_loop_fallible_impl (input)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
