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
		_ = #shutdown => { break; }
	}
}

fn implement_stream_pattern (stream_pattern: StreamEventPattern)
-> Result <proc_macro2::TokenStream>
{
	let StreamEventPattern {stream, question_token, item, handler, ..}
		= stream_pattern;

	if question_token . is_some ()
	{
		return Err (
			Error::new_spanned
			(
				question_token,
				"Fallible streams are not supported by `event_loop!`.  Use `event_loop_fallible!` instead."
			)
		);
	}

	let tokens = quote!
	{
		__item = #stream . next () => compute_graph::check_break!
		(
			compute_graph::handle_stream_output!
			(
				Some (#item) = __item =>
				{
					let _: () = #handler;
					core::ops::ControlFlow::<()>::Continue (())
				}
			)
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
		return Err (
			Error::new_spanned
			(
				question_token,
				"Fallible streams are not supported by `event_loop!`.  Use `event_loop_fallible!` instead."
			)
		);
	}

	let item_handler = quote!
	{{
		let _: () = #item_handler;
		core::ops::ControlFlow::<()>::Continue (())
	}};

	let finish_handler = finish_handler
		. map (|FinishHandler {handler, ..}| quote! (let _: () = #handler));

	let tokens = quote!
	{
		__item = #stream . next () =>
		{
			compute_graph::check_break!
			(
				compute_graph::handle_stream_output!
				(
					Some (#item) = __item => #item_handler
				)
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
								Some (#item) = __item => #item_handler
							)
						)
					),
					'__event_loop
				);
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

	quote! (#value = #future => { let _: () = #handler; })
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

fn event_loop_inner (event_patterns: Punctuated <EventPattern, Token! [,]>)
-> Result <proc_macro2::TokenStream>
{
	let implemented_event_patterns = event_patterns
		. into_iter ()
		. map (|event_pattern| implement_event_pattern (event_pattern))
		. collect::<Result <Vec <proc_macro2::TokenStream>>> ()?;

	let tokens = quote!
	{{
		let _: () = '__event_loop: loop
		{
			let _: () = tokio::select!
			(
				biased;
				#(#implemented_event_patterns),*
			);
		};
	}};

	Ok (tokens)
}

fn try_event_loop_impl (input: proc_macro::TokenStream)
-> Result <proc_macro2::TokenStream>
{
	let event_patterns = Punctuated::parse_terminated . parse (input)?;

	Ok (event_loop_inner (event_patterns)?)
}

pub fn event_loop_impl (input: proc_macro::TokenStream)
-> proc_macro::TokenStream
{
	try_event_loop_impl (input)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
