use syn::Token;
use syn::punctuated::Punctuated;
use syn::parse::{Parser, Result, Error};
use quote::{ToTokens, quote};

use crate::event_pattern::*;

fn implement_shutdown_pattern (shutdown_pattern: ShutdownEventPattern)
-> proc_macro2::TokenStream
{
	let ShutdownEventPattern {shutdown, ..} = shutdown_pattern;

	quote!
	{
		_ = #shutdown => core::ops::ControlFlow::Break (())
	}
}

fn implement_stream_pattern (stream_pattern: StreamEventPattern)
-> Result <proc_macro2::TokenStream>
{
	let StreamEventPattern {stream, question_token, item, handler, ..}
		= stream_pattern;

	if question_token . is_some ()
	{
		return Err
		(
			Error::new_spanned
			(
				question_token,
				"Fallible streams are not supported by `select!`.  Use `select_fallible!` instead."
			)
		);
	}

	let tokens = quote!
	{
		__item = #stream . next () => compute_graph::handle_stream_output!
		(
			Some (#item) = __item => #handler
		)
	};

	Ok (tokens)
}

fn implement_stream_iter_pattern (stream_iter_pattern: StreamIterEventPattern)
-> Result <proc_macro2::TokenStream>
{
	let StreamIterEventPattern
	{
		stream,
		question_token,
		item,
		item_handler,
		finish_handler,
		..
	}
		= stream_iter_pattern;

	if question_token . is_some ()
	{
		return Err
		(
			Error::new_spanned
			(
				question_token,
				"Fallible streams are not supported by `select!`.  Use `select_fallible!` instead."
			)
		);
	}

	let finish_handler = match finish_handler
	{
		None => quote! (core::ops::ControlFlow::Continue (())),
		Some (FinishHandler {handler, ..}) => handler . into_token_stream ()
	};

	let tokens = quote!
	{
		__item = #stream . next () =>
		'__select_handler: {
			if let core::ops::ControlFlow::Break (()) compute_graph::handle_stream_output!
			(
				Some (#item) = __item => #item_handler
			)
			{
				break '__select_handler core::ops::ControlFlow::Break (());
			}

			for __item
			in compute_graph::stream::ready_items (std::pin::Pin::new (&mut #stream))
			{
				if let core::ops::ControlFlow::Break (()) = compute_graph::handle_stream_output!
				(
					Some (#item) = __item => #item_handler
				)
				{
					break __select_handler core::ops::ControlFlow::Break (());
				}
			}

			#finish_handler
		}
	};

	Ok (tokens)
}

fn implement_future_pattern (future_pattern: FutureEventPattern)
-> proc_macro2::TokenStream
{
	let FutureEventPattern {value, future, handler, ..} = future_pattern;

	quote! (#value = #future => #handler)
}

fn implement_event_pattern (event_pattern: EventPattern)
-> Result <proc_macro2::TokenStream>
{
	let tokens = match event_pattern
	{
		EventPattern::Shutdown (shutdown_pattern) =>
			implement_shutdown_pattern (shutdown_pattern),
		EventPattern::Stream (stream_pattern) =>
			implement_stream_pattern (stream_pattern)?,
		EventPattern::StreamIter (stream_iter_pattern) =>
			implement_stream_iter_pattern (stream_iter_pattern)?,
		EventPattern::Future (future_pattern) =>
			implement_future_pattern (future_pattern)
	};

	Ok (tokens)
}

fn select_inner (event_patterns: Punctuated <EventPattern, Token! [,]>)
-> Result <proc_macro2::TokenStream>
{
	let implemented_event_patterns = event_patterns
		. into_iter ()
		. map (|event_pattern| implement_event_pattern (event_pattern))
		. collect::<Result <Vec <proc_macro2::TokenStream>>> ()?;

	let tokens = quote!
	{{
		let c: core::ops::ControlFlow <()> = tokio::select!
		(
			biased;
			#(#implemented_event_patterns),*
		);

		c
	}};

	Ok (tokens)
}

fn try_select_impl (input: proc_macro::TokenStream)
-> Result <proc_macro2::TokenStream>
{
	let event_patterns = Punctuated::parse_terminated . parse (input)?;

	Ok (select_inner (event_patterns)?)
}

pub fn select_impl (input: proc_macro::TokenStream)
-> proc_macro::TokenStream
{
	try_select_impl (input)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
