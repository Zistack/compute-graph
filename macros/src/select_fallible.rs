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
		_ = #shutdown => core::ops::ControlFlow::Break
		(
			compute_graph::exit_status::ExitStatus::Clean
		)
	}
}

fn implement_stream_pattern (stream_pattern: StreamEventPattern)
-> proc_macro2::TokenStream
{
	let StreamEventPattern {stream, question_token, item, handler, ..}
		= stream_pattern;

	let map_tokens = match &question_token
	{
		None => Some
		(
			quote!
			(
				. map_break (|_| compute_graph::exit_status::ExitStatus::Clean)
			)
		),
		Some (_) => None
	};

	quote!
	{
		__item = #stream . next () => compute_graph::handle_stream_output!
		(
			Some (#item) = #question_token __item => #handler
		) #map_tokens
	}
}

fn implement_stream_iter_pattern (stream_iter_pattern: StreamIterEventPattern)
-> proc_macro2::TokenStream
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

	let map_tokens = match &question_token
	{
		None => Some
		(
			quote!
			(
				. map_break (|_| compute_graph::exit_status::ExitStatus::Clean)
			)
		),
		Some (_) => None
	};

	let finish_handler = match finish_handler
	{
		None => quote! (core::ops::ControlFlow::Continue (())),
		Some (FinishHandler {handler, ..}) => handler . into_token_stream ()
	};

	quote!
	{
		__item = #stream . next () =>
		'__select_fallible_handler: {
			if let core::ops::ControlFlow::Break (b) compute_graph::handle_stream_output!
			(
				Some (#item) = #question_token __item => #item_handler
			) #map_tokens
			{
				break '__select_fallible_handler core::ops::ControlFlow::Break (b);
			}

			for __item
			in compute_graph::stream::ready_items (std::pin::Pin::new (&mut #stream))
			{
				if let core::ops::ControlFlow::Break (b) = compute_graph::handle_stream_output!
				(
					Some (#item) = #question_token __item => #item_handler
				) #map_tokens
				{
					break __select_fallible_handler core::ops::ControlFlow::Break (b);
				}
			}

			#finish_handler
		}
	}
}

fn implement_future_pattern (future_pattern: FutureEventPattern)
-> proc_macro2::TokenStream
{
	let FutureEventPattern {value, future, handler, ..} = future_pattern;

	quote! (#value = #future => #handler)
}

fn implement_event_pattern (event_pattern: EventPattern)
-> proc_macro2::TokenStream
{
	match event_pattern
	{
		EventPattern::Shutdown (shutdown_pattern) =>
			implement_shutdown_pattern (shutdown_pattern),
		EventPattern::Stream (stream_pattern) =>
			implement_stream_pattern (stream_pattern),
		EventPattern::StreamIter (stream_iter_pattern) =>
			implement_stream_iter_pattern (stream_iter_pattern),
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
	{{
		let c: core::ops::ControlFlow <compute_graph::exit_status::ExitStatus> =
			tokio::select!
		(
			biased;
			#(#implemented_event_patterns),*
		);

		c
	}}
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
