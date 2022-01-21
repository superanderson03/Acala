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

//! Autogenerated weights for module_evm
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-01-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/acala
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=module_evm
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./modules/evm/src/weights.rs
// --template=./templates/module-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for module_evm.
pub trait WeightInfo {
	fn create() -> Weight;
	fn create2() -> Weight;
	fn create_nft_contract() -> Weight;
	fn create_predeploy_contract() -> Weight;
	fn deposit_ed() -> Weight;
	fn call() -> Weight;
	fn transfer_maintainer() -> Weight;
	fn publish_contract() -> Weight;
	fn publish_free() -> Weight;
	fn enable_contract_development() -> Weight;
	fn disable_contract_development() -> Weight;
	fn set_code(c: u32, ) -> Weight;
	fn selfdestruct() -> Weight;
}

/// Weights for module_evm using the Acala node and recommended hardware.
pub struct AcalaWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for AcalaWeight<T> {
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	// Storage: EvmAccounts Accounts (r:2 w:0)
	// Storage: Balances Reserves (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: EVM Accounts (r:2 w:2)
	// Storage: EVM Codes (r:1 w:1)
	// Storage: EVM CodeInfos (r:1 w:1)
	// Storage: EVM ContractStorageSizes (r:1 w:1)
	fn create() -> Weight {
		(273_424_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(12 as Weight))
			.saturating_add(T::DbWeight::get().writes(9 as Weight))
	}
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	// Storage: EvmAccounts Accounts (r:2 w:0)
	// Storage: Balances Reserves (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: EVM Accounts (r:2 w:2)
	// Storage: EVM Codes (r:1 w:1)
	// Storage: EVM CodeInfos (r:1 w:1)
	// Storage: EVM ContractStorageSizes (r:1 w:1)
	fn create2() -> Weight {
		(265_798_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(12 as Weight))
			.saturating_add(T::DbWeight::get().writes(9 as Weight))
	}
	// Storage: EVM NetworkContractIndex (r:1 w:1)
	// Storage: EvmAccounts Accounts (r:2 w:0)
	// Storage: Balances Reserves (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: EVM Accounts (r:3 w:2)
	// Storage: EVM Codes (r:1 w:1)
	// Storage: EVM CodeInfos (r:1 w:1)
	// Storage: EVM ContractStorageSizes (r:1 w:1)
	fn create_nft_contract() -> Weight {
		(260_825_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(13 as Weight))
			.saturating_add(T::DbWeight::get().writes(10 as Weight))
	}
	// Storage: EvmAccounts Accounts (r:2 w:0)
	// Storage: System Account (r:2 w:2)
	// Storage: EVM Accounts (r:3 w:2)
	// Storage: Balances Reserves (r:2 w:2)
	// Storage: EVM Codes (r:1 w:1)
	// Storage: EVM CodeInfos (r:1 w:1)
	// Storage: EVM ContractStorageSizes (r:1 w:1)
	fn create_predeploy_contract() -> Weight {
		(265_912_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(12 as Weight))
			.saturating_add(T::DbWeight::get().writes(9 as Weight))
	}
	// Storage: EvmAccounts Accounts (r:1 w:0)
	// Storage: System Account (r:2 w:2)
	// Storage: EVM Accounts (r:1 w:0)
	fn deposit_ed() -> Weight {
		(77_717_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	// Storage: EVM Accounts (r:2 w:1)
	// Storage: EvmAccounts Accounts (r:2 w:0)
	// Storage: Balances Reserves (r:2 w:2)
	// Storage: System Account (r:2 w:2)
	// Storage: EVM Codes (r:1 w:0)
	// Storage: EVM ContractStorageSizes (r:1 w:1)
	fn call() -> Weight {
		(241_640_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(11 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	// Storage: EVM Accounts (r:1 w:1)
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	fn transfer_maintainer() -> Weight {
		(135_536_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	// Storage: EVM Accounts (r:1 w:1)
	fn publish_contract() -> Weight {
		(167_807_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: EVM Accounts (r:1 w:1)
	fn publish_free() -> Weight {
		(31_397_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Balances Reserves (r:1 w:1)
	fn enable_contract_development() -> Weight {
		(148_829_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Balances Reserves (r:1 w:1)
	fn disable_contract_development() -> Weight {
		(145_181_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: EVM Accounts (r:1 w:1)
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	// Storage: EVM CodeInfos (r:2 w:2)
	// Storage: EvmAccounts Accounts (r:2 w:0)
	// Storage: Balances Reserves (r:2 w:2)
	// Storage: System Account (r:1 w:1)
	// Storage: EVM ContractStorageSizes (r:1 w:1)
	// Storage: EVM Codes (r:0 w:2)
	fn set_code(c: u32, ) -> Weight {
		(256_455_000 as Weight)
			// Standard Error: 0
			.saturating_add((10_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(10 as Weight))
			.saturating_add(T::DbWeight::get().writes(9 as Weight))
	}
	// Storage: EvmAccounts EvmAddresses (r:1 w:0)
	// Storage: EVM Accounts (r:1 w:1)
	// Storage: EvmAccounts Accounts (r:1 w:0)
	// Storage: EVM CodeInfos (r:1 w:1)
	// Storage: EVM ContractStorageSizes (r:1 w:1)
	// Storage: IdleScheduler NextTaskId (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: IdleScheduler Tasks (r:0 w:1)
	// Storage: EVM Codes (r:0 w:1)
	fn selfdestruct() -> Weight {
		(168_422_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn create() -> Weight {
		(273_424_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(12 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
	fn create2() -> Weight {
		(265_798_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(12 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
	fn create_nft_contract() -> Weight {
		(260_825_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(13 as Weight))
			.saturating_add(RocksDbWeight::get().writes(10 as Weight))
	}
	fn create_predeploy_contract() -> Weight {
		(265_912_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(12 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
	fn deposit_ed() -> Weight {
		(77_717_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	fn call() -> Weight {
		(241_640_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(11 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
	}
	fn transfer_maintainer() -> Weight {
		(135_536_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn publish_contract() -> Weight {
		(167_807_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	fn publish_free() -> Weight {
		(31_397_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn enable_contract_development() -> Weight {
		(148_829_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn disable_contract_development() -> Weight {
		(145_181_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn set_code(c: u32, ) -> Weight {
		(256_455_000 as Weight)
			// Standard Error: 0
			.saturating_add((10_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(RocksDbWeight::get().reads(10 as Weight))
			.saturating_add(RocksDbWeight::get().writes(9 as Weight))
	}
	fn selfdestruct() -> Weight {
		(168_422_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(7 as Weight))
	}
}
