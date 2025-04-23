use compute_graph::task;
use compute_graph::task_handle::TaskHandle;
use tokio::select;
use tokio::time::{Duration, sleep};

#[task]
async fn answer_cancellable () -> Option <u32>
{
	sleep (Duration::from_millis (100)) . await;
	Some (42)
}

#[tokio::main]
#[test]
async fn get_answer_cancellable ()
{
	let mut answer_handle = answer_cancellable ();
	answer_handle . abort ();
	assert_eq! (answer_handle . await, None);

	let answer_handle = answer_cancellable ();
	assert_eq! (answer_handle . await, Some (42));
}

#[task (forking)]
async fn answer_cancellable_forking () -> Option <u32>
{
	sleep (Duration::from_millis (100)) . await;
	Some (42)
}

#[tokio::main]
#[test]
async fn get_answer_cancellable_forking ()
{
	let mut answer_handle = answer_cancellable_forking ();
	answer_handle . abort ();
	assert_eq! (answer_handle . await, None);

	let answer_handle = answer_cancellable_forking ();
	assert_eq! (answer_handle . await, Some (42));
}

#[task (shutdown = shutdown)]
async fn answer_signallable () -> Option <u32>
{
	select!
	{
		_ = shutdown => None,
		_ = sleep (Duration::from_millis (100)) => Some (42)
	}
}

#[tokio::main]
#[test]
async fn get_answer_signallable ()
{
	let mut answer_handle = answer_signallable ();
	answer_handle . abort ();
	assert_eq! (answer_handle . await, None);

	let answer_handle = answer_signallable ();
	assert_eq! (answer_handle . await, Some (42));
}

#[task (shutdown = shutdown, forking)]
async fn answer_signallable_forking () -> Option <u32>
{
	select!
	{
		_ = shutdown => None,
		_ = sleep (Duration::from_millis (100)) => Some (42)
	}
}

#[tokio::main]
#[test]
async fn get_answer_signallable_forking ()
{
	let mut answer_handle = answer_signallable_forking ();
	answer_handle . abort ();
	assert_eq! (answer_handle . await, None);

	let answer_handle = answer_signallable_forking ();
	assert_eq! (answer_handle . await, Some (42));
}
