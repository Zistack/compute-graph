#[macro_export]
macro_rules! __check_break
{
	($e: expr) =>
	{
		match $e
		{
			core::ops::ControlFlow::Continue (v) => v,
			core::ops::ControlFlow::Break (v) => { break v; }
		}
	};
	($e: expr, $label: lifetime) =>
	{
		match $e
		{
			core::ops::ControlFlow::Continue (v) => v,
			core::ops::ControlFlow::Break (v) => { break $label v; }
		}
	}
}
pub use __check_break as check_break;
