use syn::Token;
use syn::punctuated::Punctuated;
use syn::parse::{Parser, Result, Error};
use quote::quote;

use crate::event_pattern::*;

fn implement_shutdown_pattern (shutdown_pattern: ShutdownEventPattern)
-> proc_macro2::TokenStream
{
	let ShutdownEventPattern {shutdown, ..} = shutdown_pattern;

	quote!
	{
		_ = #shutdown =>
		{
			break compute_graph::exit_status::ExitStatus::Clean;
		}
	}
}

fn implement_stream_pattern (stream_pattern: StreamEventPattern)
-> proc_macro2::TokenStream
{
	let StreamEventPattern {stream, question_token, item, handler, ..}
		= stream_pattern;

	let map_tokens = match question_token
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
		__item = #stream . next () => compute_graph::check_break!
		(
			compute_graph::handle_stream_output!
			(
				Some (#item) = #question_token __item =>
				{
					let _: () = #handler;
					core::ops::ControlFlow::<compute_graph::exit_status::ExitStatus>::Continue (())
				}
			) #map_tokens
		)
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

	let map_tokens = match question_token
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

	let item_handler = quote!
	{{
		let _: () = #item_handler;
		core::ops::ControlFlow::<compute_graph::exit_status::ExitStatus>::Continue (())
	}};

	let finish_handler = finish_handler
		. map (|FinishHandler {handler, ..}| quote! (let _: () = #handler;));

	quote!
	{
		__item = #stream . next () =>
		{
			compute_graph::check_break!
			(
				compute_graph::handle_stream_output!
				(
					Some (#item) = #question_token __item => #item_handler
				) #map_tokens
			);

			for __item
			in compute_graph::stream::ready_items (std::pin::Pin::new (&mut #stream))
			{
				compute_graph::check_break!
				(
					compute_graph::capture_break!
					(
						compute_graph::check_break!
						(
							compute_graph::handle_stream_output!
							(
								Some (#item) = #question_token __item =>
									#item_handler
							) #map_tokens
						)
					),
					'__event_loop_fallible
				);
			}

			#finish_handler
		}
	}
}

fn implement_future_pattern (future_pattern: FutureEventPattern)
-> proc_macro2::TokenStream
{
	let FutureEventPattern {value, future, handler, ..} = future_pattern;

	quote! (#value = #future => { let _: () = #handler; })
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

fn event_loop_fallible_inner (event_patterns: Punctuated <EventPattern, Token! [,]>)
-> proc_macro2::TokenStream
{
	let implemented_event_patterns = event_patterns
		. into_iter ()
		. map (|event_pattern| implement_event_pattern (event_pattern));

	quote!
	{{
		let e: compute_graph::exit_status::ExitStatus =
			'__event_loop_fallible: loop
		{
			let _: () = tokio::select!
			(
				biased;
				#(#implemented_event_patterns),*
			);
		};

		e
	}}
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
