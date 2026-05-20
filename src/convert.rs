use crate::exit_status::ExitStatus;

pub trait Convert <A, B>
{
	fn convert (a: A) -> B;
}

pub struct Converter;

impl <T> Convert <T, T> for Converter
{
	fn convert (t: T) -> T { t }
}

impl Convert <(), ExitStatus> for Converter
{
	fn convert (_: ()) -> ExitStatus { ExitStatus::Clean }
}
