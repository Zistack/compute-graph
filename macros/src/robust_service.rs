use syn::{
	Ident,
	ItemFn,
	LitBool,
	FnArg,
	ReturnType,
	Token,
	parse,
	parse2,
	parse_quote
};
use syn::parse::{Nothing, Parse, ParseStream, Result, Error};
use syn::punctuated::Punctuated;
use syn_derive::{Parse, ToTokens};
use quote::{format_ident, quote};

use crate::util::scan_arg;

mod kw
{
	syn::custom_keyword! (constructor_shutdown);
	syn::custom_keyword! (service_shutdown);
	syn::custom_keyword! (preemption_interval_marker);
	syn::custom_keyword! (state_channel_sender_marker);
	syn::custom_keyword! (forking);
}

#[allow (dead_code)]
#[derive (Parse, ToTokens)]
struct ConstructorShutdownObject
{
	marker: kw::constructor_shutdown,
	eq_token: Token! [=],
	ident: Ident
}

#[allow (dead_code)]
#[derive (Parse, ToTokens)]
struct ServiceShutdownFlag
{
	marker: kw::service_shutdown,
	eq_token: Token! [=],
	flag: LitBool
}

#[allow (dead_code)]
#[derive (Parse, ToTokens)]
struct PreemptionIntervalMarker
{
	preemption_interval_marker_token: kw::preemption_interval_marker,
	eq_token: Token! [=],
	ident: Ident
}

#[allow (dead_code)]
#[derive (Parse, ToTokens)]
struct StateChannelSenderMarker
{
	state_channel_sender_marker_token: kw::state_channel_sender_marker,
	eq_token: Token! [=],
	ident: Ident
}

#[allow (dead_code)]
#[derive (Parse, ToTokens)]
struct ForkingFlag
{
	forking_token: kw::forking,
	eq_token: Token! [=],
	flag: LitBool
}

#[derive (Parse)]
enum AttrArg
{
	#[parse (peek = kw::constructor_shutdown)]
	ConstructorShutdown (ConstructorShutdownObject),
	#[parse (peek = kw::service_shutdown)]
	ServiceShutdown (ServiceShutdownFlag),
	#[parse (peek = kw::preemption_interval_marker)]
	PreemptionInterval (PreemptionIntervalMarker),
	#[parse (peek = kw::state_channel_sender_marker)]
	StateChannelSender (StateChannelSenderMarker),
	#[parse (peek = kw::forking)]
	Forking (ForkingFlag)
}

enum ShutdownSpec
{
	Constructor (ConstructorShutdownObject),
	Service (ServiceShutdownFlag),
	None
}

impl ShutdownSpec
{
	fn is_none (&self) -> bool
	{
		match self
		{
			Self::Constructor (_) => false,
			Self::Service (_) => false,
			Self::None => true
		}
	}
}

struct RobustServiceInput
{
	shutdown_spec: ShutdownSpec,
	preemption_interval_marker: Option <PreemptionIntervalMarker>,
	state_channel_sender_marker: Option <StateChannelSenderMarker>,
	forking_flag: Option <ForkingFlag>
}

impl Parse for RobustServiceInput
{
	fn parse (input: ParseStream <'_>) -> Result <Self>
	{
		let mut shutdown_spec = ShutdownSpec::None;
		let mut preemption_interval_marker = None;
		let mut state_channel_sender_marker = None;
		let mut forking_flag = None;

		for attr_arg
		in Punctuated::<AttrArg, Token! [,]>::parse_terminated (input)?
		{
			match attr_arg
			{
				AttrArg::ConstructorShutdown (constructor_shutdown_object) =>
				{
					if ! shutdown_spec . is_none ()
					{
						return Err
						(
							Error::new_spanned
							(
								constructor_shutdown_object,
								"cannot specify multiple shutdown modes"
							)
						);
					}

					shutdown_spec =
						ShutdownSpec::Constructor (constructor_shutdown_object);
				},
				AttrArg::ServiceShutdown (service_shutdown_flag) =>
				{
					if ! shutdown_spec . is_none ()
					{
						return Err
						(
							Error::new_spanned
							(
								service_shutdown_flag,
								"cannot specify multiple shutdown modes"
							)
						);
					}

					shutdown_spec =
						ShutdownSpec::Service (service_shutdown_flag);
				},
				AttrArg::PreemptionInterval (preemption_interval_marker_arg) =>
				{
					if preemption_interval_marker . is_some ()
					{
						return Err
						(
							Error::new_spanned
							(
								preemption_interval_marker_arg,
								"preemption interval marker should be specified at most once"
							)
						);
					}

					preemption_interval_marker =
						Some (preemption_interval_marker_arg);
				},
				AttrArg::StateChannelSender (state_channel_sender_marker_arg) =>
				{
					if state_channel_sender_marker . is_some ()
					{
						return Err
						(
							Error::new_spanned
							(
								state_channel_sender_marker_arg,
								"state channel sender marker should be specified at most once"
							)
						);
					}

					state_channel_sender_marker =
						Some (state_channel_sender_marker_arg);
				},
				AttrArg::Forking (forking_flag_arg) =>
				{
					if forking_flag . is_some ()
					{
						return Err
						(
							Error::new_spanned
							(
								forking_flag_arg,
								"forking flag should be specified at most once"
							)
						);
					}

					forking_flag = Some (forking_flag_arg);
				}
			}
		}

		Ok
		(
			Self
			{
				shutdown_spec,
				preemption_interval_marker,
				state_channel_sender_marker,
				forking_flag
			}
		)
	}
}

enum ShutdownConfig
{
	Constructor (Ident),
	Service,
	None
}

struct RobustServiceConfiguration
{
	shutdown_config: ShutdownConfig,
	preemption_interval_marker: Ident,
	state_channel_sender_marker: Ident,
	forking: bool
}

impl From <RobustServiceInput> for RobustServiceConfiguration
{
	fn from (input: RobustServiceInput) -> Self
	{
		let shutdown_config = match input . shutdown_spec
		{
			ShutdownSpec::Constructor (constructor_shutdown_object) =>
				ShutdownConfig::Constructor (constructor_shutdown_object . ident),
			ShutdownSpec::Service (service_shutdown_flag) if service_shutdown_flag . flag . value =>
				ShutdownConfig::Service,
			_ => ShutdownConfig::None
		};

		let preemption_interval_marker = match input . preemption_interval_marker
		{
			None => format_ident! ("preemption_interval"),
			Some (preemption_interval_marker) => preemption_interval_marker . ident
		};

		let state_channel_sender_marker = match input . state_channel_sender_marker
		{
			None => format_ident! ("state_channel"),
			Some (state_channel_sender_marker) => state_channel_sender_marker . ident
		};

		let forking = match input . forking_flag
		{
			None => false,
			Some (forking_flag) => forking_flag . flag . value
		};

		Self
		{
			shutdown_config,
			preemption_interval_marker,
			state_channel_sender_marker,
			forking
		}
	}
}

enum MacroArgType
{
	PreemptionInterval,
	StateChannelSender
}

fn match_macro
(
	preemption_marker: &Ident,
	state_channel_marker: &Ident,
	macro_ident: &Ident,
	macro_tokens: &proc_macro2::TokenStream
)
-> Result <Option <MacroArgType>>
{
	if macro_ident == preemption_marker
	{
		parse2::<Nothing> (macro_tokens . clone ())?;

		return Ok (Some (MacroArgType::PreemptionInterval));
	}

	if macro_ident == state_channel_marker
	{
		parse2::<Nothing> (macro_tokens . clone ())?;

		return Ok (Some (MacroArgType::StateChannelSender));
	}

	Ok (None)
}

fn robust_service_inner
(
	config: RobustServiceConfiguration,
	function: ItemFn
)
-> Result <proc_macro2::TokenStream>
{
	let ItemFn {attrs, vis, mut sig, block} = function;

	sig . asyncness = None;

	let mut preemption_interval_ident = None;
	let mut state_channel_sender_ident = None;

	for fn_arg in &mut sig . inputs
	{
		match fn_arg
		{
			FnArg::Receiver (receiver) =>
			{
				let ty = &receiver . ty;
				sig
					. generics
					. make_where_clause ()
					. predicates
					. push (parse_quote! (#ty: Clone + Send + 'static));
			}
			FnArg::Typed (pat_type) =>
			{
				match scan_arg
				(
					pat_type,
					|macro_ident, macro_tokens|
					match_macro
					(
						&config . preemption_interval_marker,
						&config . state_channel_sender_marker,
						macro_ident,
						macro_tokens
					)
				)?
				{
					None =>
					{
						let ty = &pat_type . ty;
						sig
							. generics
							. make_where_clause ()
							. predicates
							. push (parse_quote! (#ty: Clone + Send + 'static));
					},
					Some ((pat_ident, MacroArgType::PreemptionInterval)) =>
					{
						if preemption_interval_ident . is_some ()
						{
							return Err
							(
								Error::new_spanned
								(
									fn_arg,
									"cannot specify more than one preemption interval argument"
								)
							)
						}

						preemption_interval_ident =
							Some (pat_ident . ident . clone ());

						*pat_type . ty = parse_quote! (tokio::time::Duration);
					},
					Some ((pat_ident, MacroArgType::StateChannelSender)) =>
					{
						if state_channel_sender_ident . is_some ()
						{
							return Err
							(
								Error::new_spanned
								(
									fn_arg,
									"cannot specify more than one state channel argument"
								)
							)
						}

						state_channel_sender_ident =
							Some (pat_ident . ident . clone ());

						*pat_type . ty = parse_quote!
						(
							tokio::sync::watch::Sender
							<
								compute_graph::service_state::ServiceState
							>
						)
					}
				}
			}
		}
	}

	let constructor_output = sig . output;

	sig . output = match &config . shutdown_config
	{
		ShutdownConfig::None => parse_quote!
		(
			compute_graph::service_handle::CancellableServiceHandle
			<
				compute_graph::exit_statue::AlwaysClean
			>
		),
		_ => parse_quote!
		(
			compute_graph::service_handle::SignallableServiceHandle
			<
				compute_graph::exit_status::AlwaysClean
			>
		)
	};

	let (cancellable_task_handle_type, signallable_task_handle_type) = match config . forking
	{
		true =>
		(
			quote! (compute_graph::task_handle::ParallelCancellableTaskHandle),
			quote! (compute_graph::task_handle::ParallelSignallableTaskHandle)
		),
		false =>
		(
			quote! (compute_graph::task_handle::CancellableTaskHandle),
			quote! (compute_graph::task_handle::SignallableTaskHandle)
		)
	};

	let constructor_def = match &config . shutdown_config
	{
		ShutdownConfig::Constructor (shutdown_object) =>
		{
			let shutdown_trigger = format_ident! ("{}_trigger", shutdown_object);

			let constructor_output = match constructor_output
			{
				ReturnType::Default => quote!
				(
					-> #signallable_task_handle_type <()>
				),
				ReturnType::Type (arrow_token, ty) => quote!
				(
					#arrow_token #signallable_task_handle_type <#ty>
				)
			};

			quote!
			{
				async move || #constructor_output
				{
					let (#shutdown_trigger, #shutdown_object) =
						tokio::sync::oneshot::channel ();

					#signallable_task_handle_type::new
					(
						async move #block,
						#shutdown_trigger
					)
				}
			}
		},
		ShutdownConfig::Service =>
		{
			let constructor_output = match constructor_output
			{
				ReturnType::Default => quote!
				(
					-> #cancellable_task_handle_type <()>
				),
				ReturnType::Type (arrow_token, ty) => quote!
				(
					#arrow_token #cancellable_task_handle_type <#ty>
				)
			};

			quote!
			{
				async move || #constructor_output
				{
					#cancellable_task_handle_type::new (async move #block)
				}
			}
		},
		ShutdownConfig::None => quote!
		{
			async move || #constructor_output #block
		}
	};

	let robust_service_invocation = match
	(
		config . shutdown_config,
		preemption_interval_ident,
		state_channel_sender_ident
	)
	{
		(ShutdownConfig::None, None, None) => quote!
		{
			compute_graph::robust_service::robust_service (#constructor_def)
		},
		(ShutdownConfig::None, None, Some (state_channel_sender_ident)) => quote!
		{
			compute_graph::robust_service_with_report
			(
				#constructor_def,
				#state_channel_sender_ident
			)
		},
		(ShutdownConfig::None, Some (replacement_interval_ident), None) => quote!
		{
			compute_graph
				::robust_service
				::robust_service_with_preemptive_replacement
			(
				#constructor_def,
				#replacement_interval_ident
			)
		},
		(
			ShutdownConfig::None,
			Some (replacement_interval_ident),
			Some (state_channel_sender_ident)
		) => quote!
		{
			compute_graph
				::robust_service
				::robust_service_with_preeemptive_repacement_and_report
			(
				#constructor_def,
				#replacement_interval_ident,
				#state_channel_sender_ident
			)
		},
		(_, None, None) => quote!
		{
			compute_graph
				::robust_service
				::robust_service_with_shutdown (#constructor_def)
		},
		(_, None, Some (state_channel_sender_ident)) => quote!
		{
			compute_graph::robust_service::robust_service_with_shutdown_and_report
			(
				#constructor_def,
				#state_channel_sender_ident
			)
		},
		(_, Some (replacement_interval_ident), None) => quote!
		{
			compute_graph
				::robust_service
				::robust_service_with_shutdown_and_preemptive_replacement
			(
				#constructor_def,
				#replacement_interval_ident
			)
		},
		(
			_,
			Some (replacement_interval_ident),
			Some (state_channel_sender_ident)
		) => quote!
		{
			compute_graph
				::robust_service
				::robust_service_with_shutdown_and_preemptive_replacement_and_report
			(
				#constructor_def,
				#replacement_interval_ident,
				#state_channel_sender_ident
			)
		}
	};

	Ok (quote! { #(#attrs)* #vis #sig { #robust_service_invocation } })
}

fn try_robust_service_impl
(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream
)
-> Result <proc_macro2::TokenStream>
{
	let robust_service_configuration =
		parse::<RobustServiceInput> (attr)? . into ();

	let function = parse (item)?;

	robust_service_inner (robust_service_configuration, function)
}

pub fn robust_service_impl
(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream
)
-> proc_macro::TokenStream
{
	try_robust_service_impl (attr, item)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
