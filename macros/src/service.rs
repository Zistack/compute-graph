use syn::{
	FnArg,
	Ident,
	ItemFn,
	ReturnType,
	Token,
	parse,
	parse_quote
};
use syn::parse::{Parse, ParseStream, Result, Error};
use syn_derive::Parse;
use quote::{format_ident, quote};

mod kw
{
	syn::custom_keyword! (shutdown);
}

#[allow (dead_code)]
#[derive (Parse)]
struct ShutdownObject
{
	shutdown_marker_token: kw::shutdown,
	eq_token: Token! [=],
	ident: Ident
}

struct MaybeShutdownObject (Option <ShutdownObject>);

impl Parse for MaybeShutdownObject
{
	fn parse (input: ParseStream <'_>) -> Result <Self>
	{
		if input . peek (kw::shutdown)
		{
			Ok (MaybeShutdownObject (Some (input . parse ()?)))
		}
		else
		{
			Ok (MaybeShutdownObject (None))
		}
	}
}

fn gen_cancellable_service (function: ItemFn) -> proc_macro2::TokenStream
{
	let ItemFn {attrs, vis, mut sig, block} = function;

	let new_return_type = match sig . output
	{
		ReturnType::Default => parse_quote!
		(
			-> compute_graph::service_handle::CancellableServiceHandle <()>
		),
		ReturnType::Type (arrow_token, ty) => parse_quote!
		(
			#arrow_token compute_graph::service_handle::CancellableServiceHandle <#ty>
		)
	};

	sig . output = new_return_type;

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

	let new_return_type = match sig . output
	{
		ReturnType::Default => parse_quote!
		(
			-> compute_graph::service_handle::SignallableServiceHandle <()>
		),
		ReturnType::Type (arrow_token, ty) => parse_quote!
		(
			#arrow_token compute_graph::service_handle::SignallableServiceHandle <#ty>
		)
	};

	sig . output = new_return_type;

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

fn service_inner (shutdown_object: Option <ShutdownObject>, mut function: ItemFn)
-> Result <proc_macro2::TokenStream>
{
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

	match shutdown_object
	{
		None => Ok (gen_cancellable_service (function)),
		Some (shutdown_object) =>
			Ok (gen_signallable_service (shutdown_object . ident, function))
	}
}

fn try_service_impl
(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream
)
-> Result <proc_macro2::TokenStream>
{
	let shutdown_object: MaybeShutdownObject = parse (attr)?;
	let input_function = parse (item)?;

	service_inner (shutdown_object . 0, input_function)
}

pub fn service_impl
(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream
)
-> proc_macro::TokenStream
{
	try_service_impl (attr, item)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
