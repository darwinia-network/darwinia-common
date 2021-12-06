
pub mod raw;
// pub mod call_tracer;

pub trait ResponseFormatter {
	type Listener: Listener;
	type Response: Serialize;

	fn format(listener: Self::Listener) -> Option<Self::Response>;
}
