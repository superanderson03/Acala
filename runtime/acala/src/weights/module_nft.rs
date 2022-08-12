// This file is part of Acala.

// Copyright (C) 2020-2022 Acala Foundation.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Autogenerated weights for module_nft
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-08-12, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("acala-dev"), DB CACHE: 1024

// Executed Command:
// target/production/acala
// benchmark
// pallet
// --chain=acala-dev
// --steps=50
// --repeat=20
// --pallet=*
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --template=./templates/runtime-weight-template.hbs
// --output=./runtime/acala/src/weights/

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for module_nft.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> module_nft::WeightInfo for WeightInfo<T> {
	// Storage: OrmlNFT NextClassId (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Balances Reserves (r:1 w:1)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: OrmlNFT Classes (r:0 w:1)
	fn create_class() -> Weight {
		(70_565_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: OrmlNFT Classes (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	// Storage: Balances Reserves (r:1 w:1)
	// Storage: OrmlNFT NextTokenId (r:1 w:1)
	// Storage: OrmlNFT Tokens (r:0 w:1)
	// Storage: OrmlNFT TokensByOwner (r:0 w:1)
	fn mint(i: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 22_000
			.saturating_add((20_111_000 as Weight).saturating_mul(i as Weight))
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
			.saturating_add(T::DbWeight::get().writes((2 as Weight).saturating_mul(i as Weight)))
	}
	// Storage: OrmlNFT Classes (r:1 w:0)
	// Storage: OrmlNFT Tokens (r:1 w:1)
	// Storage: Balances Reserves (r:2 w:2)
	// Storage: System Account (r:1 w:1)
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	// Storage: OrmlNFT TokensByOwner (r:0 w:2)
	fn transfer() -> Weight {
		(95_157_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	// Storage: OrmlNFT Classes (r:1 w:1)
	// Storage: OrmlNFT Tokens (r:1 w:1)
	// Storage: Balances Reserves (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: OrmlNFT TokensByOwner (r:0 w:1)
	fn burn() -> Weight {
		(70_465_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: OrmlNFT Classes (r:1 w:1)
	// Storage: OrmlNFT Tokens (r:1 w:1)
	// Storage: Balances Reserves (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: OrmlNFT TokensByOwner (r:0 w:1)
	fn burn_with_remark(b: u32, ) -> Weight {
		(121_147_000 as Weight)
			// Standard Error: 0
			.saturating_add((2_000 as Weight).saturating_mul(b as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: OrmlNFT Classes (r:1 w:1)
	// Storage: Balances Reserves (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Proxy Proxies (r:1 w:1)
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	// Storage: OrmlNFT NextTokenId (r:0 w:1)
	fn destroy_class() -> Weight {
		(79_410_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: OrmlNFT Classes (r:1 w:1)
	fn update_class_properties() -> Weight {
		(17_178_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}
