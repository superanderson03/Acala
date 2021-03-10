// This file is part of Acala.

// Copyright (C) 2020-2021 Acala Foundation.
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


//! Autogenerated weights for module_incentives
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-02-26, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/acala
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=module_incentives
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./modules/incentives/src/weights.rs
// --template=./templates/module-weight-template.hbs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for module_incentives.
pub trait WeightInfo {
	fn deposit_dex_share() -> Weight;
	fn withdraw_dex_share() -> Weight;
	fn claim_rewards() -> Weight;
	fn update_loans_incentive_rewards(c: u32, ) -> Weight;
	fn update_dex_incentive_rewards(c: u32, ) -> Weight;
	fn update_homa_incentive_reward() -> Weight;
	fn update_dex_saving_rates(c: u32, ) -> Weight;
}

/// Weights for module_incentives using the Acala node and recommended hardware.
pub struct AcalaWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for AcalaWeight<T> {
	fn deposit_dex_share() -> Weight {
		(84_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(9 as Weight))
			.saturating_add(T::DbWeight::get().writes(9 as Weight))
	}
	fn withdraw_dex_share() -> Weight {
		(96_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	fn claim_rewards() -> Weight {
		(27_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn update_loans_incentive_rewards(c: u32, ) -> Weight {
		(479_000 as Weight)
			// Standard Error: 29_000
			.saturating_add((1_893_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
	fn update_dex_incentive_rewards(c: u32, ) -> Weight {
		(1_275_000 as Weight)
			// Standard Error: 17_000
			.saturating_add((1_632_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
	fn update_homa_incentive_reward() -> Weight {
		(2_000_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn update_dex_saving_rates(c: u32, ) -> Weight {
		(914_000 as Weight)
			// Standard Error: 21_000
			.saturating_add((1_829_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn deposit_dex_share() -> Weight {
		(84_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(9 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
	fn withdraw_dex_share() -> Weight {
		(96_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
	}
	fn claim_rewards() -> Weight {
		(27_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	fn update_loans_incentive_rewards(c: u32, ) -> Weight {
		(479_000 as Weight)
			// Standard Error: 29_000
			.saturating_add((1_893_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
	fn update_dex_incentive_rewards(c: u32, ) -> Weight {
		(1_275_000 as Weight)
			// Standard Error: 17_000
			.saturating_add((1_632_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
	fn update_homa_incentive_reward() -> Weight {
		(2_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn update_dex_saving_rates(c: u32, ) -> Weight {
		(914_000 as Weight)
			// Standard Error: 21_000
			.saturating_add((1_829_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
}
