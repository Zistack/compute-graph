use syn::{Expr, Index, Token, parse};
use syn::parse::{Parse, ParseStream, Result, Error};
use syn::punctuated::Punctuated;
use syn_derive::Parse;
use quote::quote;

#[allow (dead_code)]
#[derive (Parse)]
struct ShutdownPrefix
{
	question_token: Token! [?],
	shutdown_expr: Expr,
	comma_token: Token! [,]
}

struct JoinServicesInput
{
	shutdown_prefix: Option <ShutdownPrefix>,
	service_exprs: Punctuated <Expr, Token! [,]>
}

impl Parse for JoinServicesInput
{
	fn parse (input: ParseStream <'_>) -> Result <Self>
	{
		let shutdown_prefix = if input . peek (Token! [?])
		{
			Some (input . parse ()?)
		}
		else { None };

		let service_exprs = Punctuated::parse_terminated (input)?;

		Ok (Self {shutdown_prefix, service_exprs})
	}
}

fn join_services_inner
(
	shutdown_expr: Option <Expr>,
	mut service_exprs: Punctuated <Expr, Token! [,]>
)
-> proc_macro2::TokenStream
{
	let shutdown_branch = shutdown_expr . map
	(
		|shutdown_expr|
		quote!
		(
			async move
			{
				let _ = #shutdown_expr . await;
				std::result::Result::<(), ()>::Err (())
			},
		)
	);

	if ! service_exprs . empty_or_trailing ()
	{
		service_exprs . push_punct (<Token! [,]>::default ());
	}

	let service_idx: Vec <Index> = (0..service_exprs . len ())
		. map (|i| i . into ())
		. collect ();

	quote!
	{
		{
			let mut services = (#service_exprs);

			match tokio::try_join!
			(
				#shutdown_branch
				#(async
				{
					compute_graph::service_handle::ServiceHandle::exit_status (&mut services . #service_idx)
						. await
						. expect ("expected service handle that could still produce an output")
						. into_result ()
				}),*
			)
			{
				std::result::Result::Ok (_) =>
				(
					#(compute_graph::service_handle::ServiceHandle::take_output (&mut services . #service_idx)
						. expect ("expected completed service"),)*
				),
				std::result::Result::Err (()) =>
				{
					#(compute_graph::service_handle::ServiceHandle::shutdown (&mut services . #service_idx);)*

					tokio::join! (#(services . #service_idx),*)
				}
			}
		}
	}
}

fn try_join_services_impl (input: proc_macro::TokenStream)
-> Result <proc_macro2::TokenStream>
{
	let JoinServicesInput {shutdown_prefix, service_exprs} = parse (input)?;

	let shutdown_expr = shutdown_prefix . map (|prefix| prefix . shutdown_expr);

	Ok (join_services_inner (shutdown_expr, service_exprs))
}

pub fn join_services_impl (input: proc_macro::TokenStream)
-> proc_macro::TokenStream
{
	try_join_services_impl (input)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
