#[macro_export]
macro_rules! __handle_sink_result
{
	($result: expr) =>
	{
		match $result
		{
			std::result::Result::Ok (()) =>
				core::ops::ControlFlow::Continue (()),
			std::result::Result::Err (sink_error) =>
			{
				tracing::event!
				(
					tracing::Level::WARN,
					error = %sink_error,
					"sink rejected item"
				);
				core::ops::ControlFlow::Break (())
			}
		}
	};
	(? $result: expr) =>
	{
		match $result
		{
			std::result::Result::Ok (()) =>
				core::ops::ControlFlow::Continue (()),
			std::result::Result::Err (sink_error) =>
			{
				tracing::event!
				(
					tracing::Level::WARN,
					error = %sink_error,
					"sink rejected item"
				);
				core::ops::ControlFlow::Break
				(
					compute_graph::exit_status::ExitStatus::Spurious
				)
			}
		}
	}
}
pub use __handle_sink_result as handle_sink_result;
