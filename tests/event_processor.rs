use compute_graph::event_processor;
use futures::sink::drain;
use futures::stream::iter;

#[event_processor]
async fn take_an_input <IS, II> (input_stream: input! (IS -> II))
-> Option <II>
{
	input_stream . next () . await
}

#[tokio::main]
#[test]
async fn input_type ()
{
	let mut stream = iter (0..3);

	assert_eq! (take_an_input (&mut stream) . await, Some (0));
	assert_eq! (take_an_input (&mut stream) . await, Some (1));
	assert_eq! (take_an_input (&mut stream) . await, Some (2));
	assert_eq! (take_an_input (&mut stream) . await, None);
}

#[event_processor]
async fn transform_an_input <IS> (input_stream: input! (IS -> impl Into <i128>))
-> Option <i128>
{
	input_stream . next () . await . map (|v| v . into ())
}

#[tokio::main]
#[test]
async fn input_traits ()
{
	let mut stream = iter (0..3);

	assert_eq! (transform_an_input (&mut stream) . await, Some (0));
	assert_eq! (transform_an_input (&mut stream) . await, Some (1));
	assert_eq! (transform_an_input (&mut stream) . await, Some (2));
	assert_eq! (transform_an_input (&mut stream) . await, None);
}

#[event_processor]
async fn feed_an_output <OS, OI>
(
	output_stream: output! (OS <- OI),
	output_item: OI
)
{
	let _ = output_stream . feed (output_item) . await;
}

#[tokio::main]
#[test]
async fn output_type ()
{
	let mut sink = drain ();

	feed_an_output (&mut sink, 0) . await;
	feed_an_output (&mut sink, 1) . await;
	feed_an_output (&mut sink, 2) . await;
}

#[event_processor]
async fn feed_an_output_with_trait <OS, OI>
(
	output_stream: output! (OS <- OI: std::fmt::Display),
	output_item: OI
)
{
	let _ = output_item . to_string ();
	let _ = output_stream . feed (output_item) . await;
}

#[tokio::main]
#[test]
async fn output_traits ()
{
	let mut sink = drain ();

	feed_an_output_with_trait (&mut sink, 0) . await;
	feed_an_output_with_trait (&mut sink, 1) . await;
	feed_an_output_with_trait (&mut sink, 2) . await;
}

#[event_processor]
async fn assert_and_pass <IS, OS>
(
	source: input! (IS -> u32),
	(start, end): (u32, u32),
	sink: output! (OS <- u32)
)
-> bool
{
	for i in start..end
	{
		match source . next () . await
		{
			Some (n) if n == i => if let Err (_) = sink . feed (n) . await
			{
				return false;
			},
			_ => return false
		}
	}

	source . next () . await . is_none ()
}

#[tokio::main]
#[test]
async fn many_varied_args ()
{
	let stream = iter (0..3);
	let sink = drain ();

	assert! (assert_and_pass (stream, (0, 3), sink) . await);
}
