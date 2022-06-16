// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
#![crate_type = "proc-macro"]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::{quote, quote_spanned};
use sha3::{Digest, Keccak256};
use syn::{parse_macro_input, spanned::Spanned, Expr, ExprLit, Ident, ItemEnum, Lit};

#[proc_macro_attribute]
pub fn selector(_: TokenStream, input: TokenStream) -> TokenStream {
	let item = parse_macro_input!(input as ItemEnum);

	let ItemEnum { attrs, vis, enum_token, ident, variants, .. } = item;

	let mut ident_expressions: Vec<Ident> = vec![];
	let mut variant_expressions: Vec<Expr> = vec![];
	for variant in variants {
		if let Some((_, Expr::Lit(ExprLit { lit, .. }))) = variant.discriminant {
			if let Lit::Str(lit_str) = lit {
				let selector = u32::from_be_bytes(
					Keccak256::digest(lit_str.value().as_ref())[..4].try_into().unwrap(),
				);
				ident_expressions.push(variant.ident);
				variant_expressions.push(Expr::Lit(ExprLit {
					lit: Lit::Verbatim(Literal::u32_unsuffixed(selector)),
					attrs: Default::default(),
				}));
			} else {
				return quote_spanned! {
					lit.span() => compile_error("Not literal string");
				}
				.into();
			}
		} else {
			return quote_spanned! {
				variant.span() => compile_error("Only literal string allowed");
			}
			.into();
		}
	}

	(quote! {
		#(#attrs)*
		#vis #enum_token #ident {
			#(
				#ident_expressions = #variant_expressions,
			)*
		}

		impl #ident {
			pub fn from_u32(value: u32) -> Result<Self, PrecompileFailure> {
				match value {
					#(#variant_expressions => Ok(#ident::#ident_expressions),)*
					_ => Err(PrecompileFailure::Revert {
						exit_status: ExitRevert::Reverted,
						output: b"Unknown action".to_vec(),
						cost: 0,
					})
				}
			}
		}
	})
	.into()
}
