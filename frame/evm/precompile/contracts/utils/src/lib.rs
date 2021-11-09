#![cfg_attr(not(feature = "std"), no_std)]

pub use darwinia_evm_precompile_utils_macro::selector;
use evm::ExitError;

const ACTION_LEN: usize = 4;

#[derive(Clone, Copy, Debug)]
pub struct DvmInputParser<'a> {
	pub input: &'a [u8],
	pub selector: u32,
}

impl<'a> DvmInputParser<'a> {
	pub fn new(input: &'a [u8]) -> Result<Self, ExitError> {
		if input.len() < ACTION_LEN {
			return Err(ExitError::Other("input length less than 4 bytes".into()));
		}

		let mut buffer = [0u8; ACTION_LEN];
		buffer.copy_from_slice(&input[0..ACTION_LEN]);
		let selector = u32::from_be_bytes(buffer);
		Ok(Self {
			input: &input[ACTION_LEN..],
			selector,
		})
	}
}
