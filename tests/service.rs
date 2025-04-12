use compute_graph::service;
use compute_graph::exit_status::AlwaysClean;
use compute_graph::service_handle::ServiceHandle;
use tokio::time::{error::Elapsed, Duration, sleep, timeout};

#[service (shutdown = shutdown)]
async fn must_be_shut_down () -> AlwaysClean
{
	let _ = shutdown . await;

	AlwaysClean::new (())
}

#[tokio::main]
#[test]
async fn shutdown_service () -> Result <(), Elapsed>
{
	let mut handle = must_be_shut_down ();

	sleep (Duration::from_millis (100)) . await;

	handle . shutdown ();

	timeout (Duration::from_millis (200), handle) . await?;

	Ok (())
}

#[service]
async fn may_be_cancelled () -> AlwaysClean
{
	sleep (Duration::from_millis (100)) . await;

	AlwaysClean::new (())
}

#[tokio::main]
#[test]
async fn cancel_service () -> Result <(), Elapsed>
{
	let mut handle = may_be_cancelled ();

	sleep (Duration::from_millis (90)) . await;

	handle . shutdown ();

	timeout (Duration::from_millis (200), handle) . await?;

	Ok (())
}

#[tokio::main]
#[test]
async fn multi_service () -> Result <(), Elapsed>
{
	let mut shutdown_handle = must_be_shut_down ();
	let cancel_handle = may_be_cancelled ();

	timeout
	(
		Duration::from_millis (200),
		async move
		{
			cancel_handle . await;
			shutdown_handle . shutdown ();
			shutdown_handle . await;
		}
	) . await
}
