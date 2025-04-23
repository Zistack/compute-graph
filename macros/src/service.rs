use darling::{FromMeta, Error};
use darling::ast::NestedMeta;
use syn::{
	FnArg,
	Ident,
	ItemFn,
	parse,
	parse_quote
};
use syn::parse::Result;
use quote::{format_ident, quote};

use crate::util::map_return_type;

#[derive (FromMeta)]
struct ServiceInput
{
	shutdown: Option <Ident>
}

fn gen_cancellable_service (function: ItemFn) -> proc_macro2::TokenStream
{
	let ItemFn {attrs, vis, mut sig, block} = function;

	sig . output = map_return_type
	(
		sig . output,
		|ty| parse_quote!
		(
			compute_graph::service_handle::CancellableServiceHandle <#ty>
		)
	);

	quote!
	{
		#(#attrs)*
		#vis #sig
		{
			compute_graph::service_handle::CancellableServiceHandle::new
			(
				tokio::task::spawn (async move #block)
			)
		}
	}
}

fn gen_signallable_service (shutdown_object: Ident, function: ItemFn)
-> proc_macro2::TokenStream
{
	let ItemFn {attrs, vis, mut sig, block} = function;

	let shutdown_trigger = format_ident! ("{}_trigger", shutdown_object);

	sig . output = map_return_type
	(
		sig . output,
		|ty| parse_quote!
		(
			compute_graph::service_handle::SignallableServiceHandle <#ty>
		)
	);

	quote!
	{
		#(#attrs)*
		#vis #sig
		{
			let (#shutdown_trigger, mut #shutdown_object) =
				tokio::sync::oneshot::channel ();

			compute_graph::service_handle::SignallableServiceHandle::new
			(
				tokio::task::spawn (async move #block),
				#shutdown_trigger
			)
		}
	}
}

fn service_inner (service_input: ServiceInput, mut function: ItemFn)
-> Result <proc_macro2::TokenStream>
{
	function . sig . asyncness = None;

	for fn_arg in &function . sig . inputs
	{
		let arg_type = match fn_arg
		{
			FnArg::Receiver (receiver) => &*receiver . ty,
			FnArg::Typed (pat_type) => &*pat_type . ty
		};

		function
			. sig
			. generics
			. make_where_clause ()
			. predicates
			. push (parse_quote! (#arg_type: Send + 'static));
	}

	match service_input . shutdown
	{
		None => Ok (gen_cancellable_service (function)),
		Some (shutdown_object) =>
			Ok (gen_signallable_service (shutdown_object, function))
	}
}

pub fn service_impl
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
		NestedMeta::parse_meta_list (attr . into ()) . map_err (|e| e . into ())
	);

	let service_input = attr_args . and_then
	(
		|attr_args| errors . handle_in (|| ServiceInput::from_list (&attr_args))
	);

	let function = errors . handle_in
	(
		|| parse (item) . map_err (|e| e . into ())
	);

	if let (Some (service_input), Some (function)) = (service_input, function)
	{
		if let Some (generated_function) = errors . handle_in
		(
			||
			service_inner (service_input, function) . map_err (|e| e . into ())
		)
		{
			errors . finish () . unwrap ();
			return generated_function . into ();
		}
	}

	errors . finish () . unwrap_err () . write_errors () . into ()
}
