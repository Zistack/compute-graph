#[macro_export]
macro_rules! __capture_break
{
	($code: expr) =>
	{
		'__captured: {
			let b = loop
			{
				let c = $code;

				break '__captured core::ops::ControlFlow::Continue (c);
			};

			core::ops::ControlFlow::Break (b)
		}
	}
}
pub use __capture_break as capture_break;
