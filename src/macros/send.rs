#[macro_export]
macro_rules! __send
{
	($sink: ident, $item: expr) =>
	{
		$crate::handle_sink_result!
		(
			futures::SinkExt::send (&mut $sink, $item) . await
		)
	};
	($sink: ident ?, $item: expr) =>
	{
		$crate::handle_sink_result!
		(
			?futures::SinkExt::send (&mut $sink, $item) . await
		)
	}
}
pub use __send as send;
