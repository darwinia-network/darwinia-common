// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// #![cfg_attr(not(feature = "std"), no_std)]

use ethabi::{
	decode,
	param_type::{ParamType, Reader},
	token::Token,
	Error,
};

pub fn decode_params(types: &[&'static str], data: &[u8]) -> Result<Vec<Token>, Error> {
	let types: Vec<ParamType> =
		types.iter().map(|t| Reader::read(&t)).collect::<Result<_, Error>>()?;

	let tokens = decode(&types, data).map_err(|_| Error::InvalidData)?;
	debug_assert_eq!(types.len(), tokens.len());
	Ok(tokens)
}

pub fn encode_params(types: &[String], values: &[String], lenient: bool) {
	todo!();
}
