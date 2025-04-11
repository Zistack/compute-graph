use proc_macro::TokenStream;

mod event_pattern;

mod event_processor;

#[proc_macro_attribute]
pub fn event_processor (attr: TokenStream, item: TokenStream) -> TokenStream
{
	event_processor::event_processor_impl (attr, item)
}

mod service;

#[proc_macro_attribute]
pub fn service (attr: TokenStream, item: TokenStream) -> TokenStream
{
	service::service_impl (attr, item)
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
