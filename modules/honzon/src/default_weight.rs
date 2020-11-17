//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn authorize() -> Weight {
		(54_182_000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn unauthorize() -> Weight {
		(51_443_000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn unauthorize_all(c: u32) -> Weight {
		(79_165_000 as Weight).saturating_add(DbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
	fn adjust_loan() -> Weight {
		(544_147_000 as Weight)
			.saturating_add(DbWeight::get().reads(24 as Weight))
			.saturating_add(DbWeight::get().writes(10 as Weight))
	}
	fn transfer_loan_from() -> Weight {
		(682_312_000 as Weight)
			.saturating_add(DbWeight::get().reads(21 as Weight))
			.saturating_add(DbWeight::get().writes(8 as Weight))
	}
}
