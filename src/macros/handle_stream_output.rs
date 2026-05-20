#[macro_export]
macro_rules! __handle_stream_output
{
	(Some ($item: ident) = $output: expr => $handler: expr) =>
	{
		match $output
		{
			core::option::Option::Some ($item) => $handler,
			core::option::Option::None =>
			{
				use compute_graph::convert::Convert;
				core::ops::ControlFlow::Break
				(
					compute_graph::convert::Converter::convert (())
				)
			}
		}
	};
	(Some ($item: ident) = ? $output: expr => $handler: expr) =>
	{
		match $output
		{
			core::option::Option::Some ($item) => $handler,
			core::option::Option::None => core::ops::ControlFlow::Break
			(
				compute_graph::exit_status::ExitStatus::Spurious
			)
		}
	}
}
pub use __handle_stream_output as handle_stream_output;
