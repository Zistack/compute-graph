#[macro_export]
macro_rules! __next
{
	($stream: ident -> $item: ident => $handler: expr) =>
	{
		$crate::handle_stream_output!
		(
			Some ($item) = futures::StreamExt::next (&mut $stream) . await => $handler
		)
	};
	($stream: ident ? -> $item: ident => $handler: expr) =>
	{
		$crate::handle_stream_output!
		(
			Some ($item) = ?futures::StreamExt::next (&mut $stream) . await => $handler
		)
	}
}
pub use __next as next;
