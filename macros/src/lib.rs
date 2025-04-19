use proc_macro::TokenStream;

mod util;
mod event_pattern;

mod expand_streams;

#[proc_macro_attribute]
pub fn expand_streams (attr: TokenStream, item: TokenStream) -> TokenStream
{
	expand_streams::expand_streams_impl (attr, item)
}

mod service;

#[proc_macro_attribute]
pub fn service (attr: TokenStream, item: TokenStream) -> TokenStream
{
	service::service_impl (attr, item)
}

mod robust_service;

#[proc_macro_attribute]
pub fn robust_service (attr: TokenStream, item: TokenStream) -> TokenStream
{
	robust_service::robust_service_impl (attr, item)
}

mod select;

#[proc_macro]
pub fn select (input: TokenStream) -> TokenStream
{
	select::select_impl (input)
}

mod select_fallible;

#[proc_macro]
pub fn select_fallible (input: TokenStream) -> TokenStream
{
	select_fallible::select_fallible_impl (input)
}

mod event_loop;

#[proc_macro]
pub fn event_loop (input: TokenStream) -> TokenStream
{
	event_loop::event_loop_impl (input)
}

mod event_loop_fallible;

#[proc_macro]
pub fn event_loop_fallible (input: TokenStream) -> TokenStream
{
	event_loop_fallible::event_loop_fallible_impl (input)
}

mod join_services;

#[proc_macro]
pub fn join_services (input: TokenStream) -> TokenStream
{
	join_services::join_services_impl (input)
}
