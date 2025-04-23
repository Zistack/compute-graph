use darling::{FromMeta, Error};
use darling::ast::NestedMeta;
use darling::util::Flag;
use syn::{Ident, ItemFn, parse, parse_quote};
use quote::{format_ident, quote};

use crate::util::map_return_type;

#[derive (FromMeta)]
struct TaskInput
{
	shutdown: Option <Ident>,
	forking: Flag
}

fn gen_cancellable_task (function: ItemFn, forking: bool)
-> proc_macro2::TokenStream
{
	let ItemFn {attrs, vis, mut sig, block} = function;

	let (task_handle_type, new_return_type) = match forking
	{
		false =>
		{
			let task_handle_type = quote!
			(
				compute_graph::task_handle::CancellableTaskHandle
			);

			let new_return_type = map_return_type
			(
				sig . output,
				|ty| parse_quote!
				(
					#task_handle_type
					<
						impl std::future::Future <Output = #ty>
					>
				)
			);

			(task_handle_type, new_return_type)
		},
		true =>
		{
			let task_handle_type = quote!
			(
				compute_graph::task_handle::ParallelCancellableTaskHandle
			);

			let new_return_type = map_return_type
			(
				sig . output,
				|ty| parse_quote! (#task_handle_type <#ty>)
			);

			(task_handle_type, new_return_type)
		}
	};

	sig . output = new_return_type;

	quote!
	{
		#(#attrs)*
		#vis #sig
		{
			#task_handle_type::new (async move #block)
		}
	}
}

fn gen_signallable_task
(
	function: ItemFn,
	shutdown_object: Ident,
	forking: bool
)
-> proc_macro2::TokenStream
{
	let ItemFn {attrs, vis, mut sig, block} = function;

	let shutdown_trigger = format_ident! ("{}_trigger", shutdown_object);

	let (task_handle_type, new_return_type) = match forking
	{
		false =>
		{
			let task_handle_type = quote!
			(
				compute_graph::task_handle::SignallableTaskHandle
			);

			let new_return_type = map_return_type
			(
				sig . output,
				|ty| parse_quote!
				(
					#task_handle_type
					<
						impl std::future::Future <Output = #ty>
					>
				)
			);

			(task_handle_type, new_return_type)
		},
		true =>
		{
			let task_handle_type = quote!
			(
				compute_graph::task_handle::ParallelSignallableTaskHandle
			);

			let new_return_type = map_return_type
			(
				sig . output,
				|ty| parse_quote! (#task_handle_type <#ty>)
			);

			(task_handle_type, new_return_type)
		}
	};

	sig . output = new_return_type;

	quote!
	{
		#(#attrs)*
		#vis #sig
		{
			let (#shutdown_trigger, #shutdown_object) =
				tokio::sync::oneshot::channel ();

			#task_handle_type::new (async move #block, #shutdown_trigger)
		}
	}
}

fn task_inner
(
	mut function: ItemFn,
	shutdown_object: Option <Ident>,
	forking: bool
)
-> proc_macro2::TokenStream
{
	function . sig . asyncness = None;

	match shutdown_object
	{
		None => gen_cancellable_task (function, forking),
		Some (shutdown_object) =>
			gen_signallable_task (function, shutdown_object, forking)
	}
}

pub fn task_impl
(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream
)
-> proc_macro::TokenStream
{
	let mut errors = Error::accumulator ();

	let attr_args = errors . handle_in
	(
		||
		NestedMeta::parse_meta_list (attr . into ())
			. map_err (|e| e . into ())
	);

	let task_input = attr_args . and_then
	(
		|attr_args| errors . handle_in (|| TaskInput::from_list (&attr_args))
	);

	let function = errors . handle_in
	(
		|| parse (item) . map_err (|e| e . into ())
	);

	match (task_input, function)
	{
		(Some (TaskInput {shutdown, forking}), Some (function)) =>
		{
			errors . finish () . unwrap ();

			task_inner (function, shutdown, forking . is_present ()) . into ()
		},
		_ => errors . finish () . unwrap_err () . write_errors () . into ()
	}
}
