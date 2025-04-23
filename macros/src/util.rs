use proc_macro2::TokenStream;
use syn::{Ident, Pat, PatIdent, PatType, ReturnType, Type, Token, parse_quote};
use syn::parse::Result;

pub fn scan_arg <M, T> (arg: &mut PatType, mut matcher: M)
-> Result <Option <(&mut PatIdent, T)>>
where M: FnMut (&Ident, &TokenStream) -> Result <Option <T>>,
{
	let pat_ident = match &mut *arg . pat
	{
		Pat::Ident (pat_ident) => pat_ident,
		_ => return Ok (None)
	};

	let (macro_ident, macro_tokens) = match &*arg . ty
	{
		Type::Macro (type_macro) =>
		{
			let macro_ident = match type_macro . mac . path . get_ident ()
			{
				Some (macro_ident) => macro_ident,
				None => return Ok (None)
			};

			let macro_tokens = &type_macro . mac . tokens;

			(macro_ident, macro_tokens)
		},
		_ => return Ok (None)
	};

	Ok
	(
		matcher (macro_ident, macro_tokens)?
			. map (|parsed_tokens| (pat_ident, parsed_tokens))
	)
}

pub fn map_return_type <F> (output: ReturnType, map: F) -> ReturnType
where F: FnOnce (Type) -> Type
{
	match output
	{
		ReturnType::Default =>
		{
			let ty = map (parse_quote! (()));
			ReturnType::Type (<Token! [->]>::default (), Box::new (ty))
		},
		ReturnType::Type (arrow_token, ty) =>
		{
			let ty = map (*ty);
			ReturnType::Type (arrow_token, Box::new (ty))
		}
	}
}
