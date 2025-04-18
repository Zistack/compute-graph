use syn::{
	FnArg,
	Ident,
	ItemFn,
	Type,
	TypeImplTrait,
	TypeParamBound,
	Token,
	parse,
	parse2,
	parse_quote
};
use syn::punctuated::Punctuated;
use syn::parse::{Parse, ParseStream, Result, Error};
use syn_derive::Parse;
use quote::{ToTokens, format_ident};

use crate::util::scan_arg;

mod kw
{
	syn::custom_keyword! (input_marker);
	syn::custom_keyword! (output_marker);
}

#[derive (Parse)]
enum InputItemSpec
{
	#[parse (peek = Token! [impl])]
	Traits (TypeImplTrait),
	Type (Type)
}

#[allow (dead_code)]
#[derive (Parse)]
struct InputPortSpec
{
	input_param: Ident,
	arrow_token: Token! [->],
	input_item_spec: InputItemSpec
}

#[allow (dead_code)]
struct OutputItemSpec
{
	output_item_type: Type,
	colon_token: Option <Token! [:]>,
	output_item_traits: Option <Punctuated <TypeParamBound, Token! [+]>>
}

impl Parse for OutputItemSpec
{
	fn parse (input: ParseStream <'_>) -> Result <Self>
	{
		let output_item_type = input . parse ()?;

		let (colon_token, output_item_traits) = if input . peek (Token! [:])
		{(
			Some (input . parse ()?),
			Some (Punctuated::parse_terminated (input)?)
		)}
		else
		{
			(None, None)
		};

		Ok (Self {output_item_type, colon_token, output_item_traits})
	}
}

#[allow (dead_code)]
#[derive (Parse)]
struct OutputPortSpec
{
	output_param: Ident,
	arrow_token: Token! [<-],
	output_item_spec: OutputItemSpec
}

enum PortSpec
{
	Input (InputPortSpec),
	Output (OutputPortSpec)
}

impl From <InputPortSpec> for PortSpec
{
	fn from (input_port_spec: InputPortSpec) -> Self
	{
		Self::Input (input_port_spec)
	}
}

impl From <OutputPortSpec> for PortSpec
{
	fn from (output_port_spec: OutputPortSpec) -> Self
	{
		Self::Output (output_port_spec)
	}
}

#[allow (dead_code)]
#[derive (Parse)]
enum MarkerSpec
{
	#[parse (peek = kw::input_marker)]
	Input
	{
		input_marker_token: kw::input_marker,
		eq_token: Token! [=],
		ident: Ident
	},
	#[parse (peek = kw::output_marker)]
	Output
	{
		output_marker_token: kw::output_marker,
		eq_token: Token! [=],
		ident: Ident
	}
}

struct ExpandStreamsMarkers
{
	input_marker: Ident,
	output_marker: Ident
}

impl Parse for ExpandStreamsMarkers
{
	fn parse (input: ParseStream <'_>) -> Result <Self>
	{
		let mut input_marker = Option::None;
		let mut output_marker = Option::None;

		let marker_specifications =
			Punctuated::<MarkerSpec, Token! [,]>::parse_terminated (input)?;

		for marker_specification in marker_specifications
		{
			match marker_specification
			{
				MarkerSpec::Input {ident, ..} =>
				{
					if let Some (ident) = input_marker
					{
						return Err
						(
							Error::new_spanned
							(
								ident,
								"input marker has already been specified"
							)
						);
					}
					else
					{
						input_marker = Some (ident);
					}
				},
				MarkerSpec::Output {ident, ..} =>
				{
					if let Some (ident) = output_marker
					{
						return Err
						(
							Error::new_spanned
							(
								ident,
								"output_marker has already been specified"
							)
						);
					}
					else
					{
						output_marker = Some (ident);
					}
				}
			}
		}

		let ret = Self
		{
			input_marker: input_marker . unwrap_or (format_ident! ("input")),
			output_marker: output_marker . unwrap_or (format_ident! ("output"))
		};

		Ok (ret)
	}
}

fn match_macro
(
	markers: &ExpandStreamsMarkers,
	macro_ident: &Ident,
	macro_tokens: &proc_macro2::TokenStream
)
-> Result <Option <PortSpec>>
{
	if macro_ident == &markers . input_marker
	{
		return Ok (Some (PortSpec::from (parse2::<InputPortSpec> (macro_tokens . clone ())?)))
	}

	if macro_ident == &markers . output_marker
	{
		return Ok (Some (PortSpec::from (parse2::<OutputPortSpec> (macro_tokens . clone ())?)))
	}

	Ok (None)
}

fn expand_streams_inner
(
	expand_streams_markers: ExpandStreamsMarkers,
	mut function: ItemFn
)
-> Result <proc_macro2::TokenStream>
{
	for fn_arg in &mut function . sig . inputs
	{
		let pat_type = match fn_arg
		{
			FnArg::Typed (pat_type) => pat_type,
			_ => continue
		};

		let (pat_ident, port_spec) = match scan_arg
		(
			pat_type,
			|ident, tokens| match_macro (&expand_streams_markers, ident, tokens)
		)?
		{
			Some (arg_info) => arg_info,
			None => continue
		};

		pat_ident . mutability = Some (<Token! [mut]>::default ());

		match port_spec
		{
			PortSpec::Input (input_port_spec) =>
			{
				let InputPortSpec {input_param, input_item_spec, ..} =
					input_port_spec;

				let predicates = &mut function
					. sig
					. generics
					. make_where_clause ()
					. predicates;

				let stream_ext_bound: TypeParamBound = match input_item_spec
				{
					InputItemSpec::Traits (input_item_traits) =>
					{
						let input_item_bounds = input_item_traits . bounds;

						predicates . push
						(
							parse_quote!
							(
								#input_param::Item: std::marker::Send
									+ #input_item_bounds
							)
						);

						parse_quote! (futures::StreamExt)
					},
					InputItemSpec::Type (input_item_type) =>
					{
						predicates . push
						(
							parse_quote! (#input_item_type: std::marker::Send)
						);

						parse_quote!
						(
							futures::StreamExt <Item = #input_item_type>
						)
					}
				};

				predicates . push
				(
					parse_quote!
					(
						#input_param: std::marker::Unpin
							+ #stream_ext_bound
							+ std::fmt::Debug
					)
				);

				*pat_type . ty = parse_quote! (#input_param);
			},
			PortSpec::Output (output_port_spec) =>
			{
				let OutputPortSpec
				{
					output_param,
					output_item_spec,
					..
				} = output_port_spec;
				let OutputItemSpec {output_item_type, output_item_traits, ..} =
					output_item_spec;

				let predicates = &mut function
					. sig
					. generics
					. make_where_clause ()
					. predicates;

				predicates . push
				(
					parse_quote!
					(
						#output_param: std::marker::Unpin
							+ futures::SinkExt <#output_item_type>
							+ std::fmt::Debug
					)
				);

				match output_item_traits
				{
					None => predicates . push
					(
						parse_quote! (#output_item_type: std::marker::Send)
					),
					Some (output_item_traits) => predicates . push
					(
						parse_quote!
						(
							#output_item_type: std::marker::Send
								+ #output_item_traits
						)
					)
				}

				predicates . push
				(
					parse_quote!
					(
						<#output_param as futures::Sink <#output_item_type>>::Error:
							std::fmt::Display
					)
				);

				*pat_type . ty = parse_quote! (#output_param);
			}
		}
	}

	Ok (function . into_token_stream ())
}

fn try_expand_streams_impl
(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream
)
-> Result <proc_macro2::TokenStream>
{
	let expand_streams_markers = parse (attr)?;
	let input_function = parse (item)?;

	expand_streams_inner (expand_streams_markers, input_function)
}

pub fn expand_streams_impl
(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream
)
-> proc_macro::TokenStream
{
	try_expand_streams_impl (attr, item)
		. unwrap_or_else (Error::into_compile_error)
		. into ()
}
