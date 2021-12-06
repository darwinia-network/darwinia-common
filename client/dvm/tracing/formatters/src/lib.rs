pub mod blockscout;
pub mod call_tracer;
pub mod raw;

pub use blockscout::Formatter as Blockscout;
pub use call_tracer::Formatter as CallTracer;
pub use raw::Formatter as Raw;

pub trait ResponseFormatter {
	type Listener: Listener;
	type Response: Serialize;

	fn format(listener: Self::Listener) -> Option<Self::Response>;
}
