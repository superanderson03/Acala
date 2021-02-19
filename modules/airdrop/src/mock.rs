//! Mocks for the airdrop module.

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, parameter_types};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};

pub type AccountId = u128;
pub type BlockNumber = u64;

pub const ALICE: AccountId = 0;
pub const BOB: AccountId = 1;
pub const CHARLIE: AccountId = 2;
pub const ACA: AirDropCurrencyId = AirDropCurrencyId::ACA;
pub const KAR: AirDropCurrencyId = AirDropCurrencyId::KAR;

mod airdrop {
	pub use super::super::*;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}

impl Config for Runtime {
	type Event = Event;
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, Call, u32, ()>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Storage, Config, Event<T>},
		AirDrop: airdrop::{Module, Call, Storage, Event<T>, Config<T>},
	}
);

pub type Airdrop = Module<Runtime>;

pub struct ExtBuilder();

impl Default for ExtBuilder {
	fn default() -> Self {
		Self()
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		airdrop::GenesisConfig::<Runtime> {
			airdrop_accounts: vec![(CHARLIE, KAR, 100), (CHARLIE, KAR, 50), (CHARLIE, ACA, 80)],
		}
		.assimilate_storage(&mut t)
		.unwrap();
		t.into()
	}
}
