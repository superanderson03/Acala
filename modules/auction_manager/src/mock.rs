//! Mocks for the auction manager module.

#![cfg(test)]

use super::*;
use frame_support::{
	impl_outer_dispatch, impl_outer_event, impl_outer_origin, ord_parameter_types, parameter_types,
	traits::GenesisBuild,
};
use frame_system::EnsureSignedBy;
use orml_traits::parameter_type_with_key;
use primitives::{Balance, CurrencyId, TokenSymbol, TradingPair};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::IdentityLookup,
	FixedPointNumber, ModuleId,
};
use sp_std::cell::RefCell;
use support::{EmergencyShutdown, Price, PriceProvider, Rate};

pub type AccountId = u128;
pub type BlockNumber = u64;
pub type AuctionId = u32;
pub type Amount = i64;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;
pub const ACA: CurrencyId = CurrencyId::Token(TokenSymbol::ACA);
pub const AUSD: CurrencyId = CurrencyId::Token(TokenSymbol::AUSD);
pub const BTC: CurrencyId = CurrencyId::Token(TokenSymbol::XBTC);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;

mod auction_manager {
	pub use super::super::*;
}

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

impl_outer_dispatch! {
	pub enum Call for Runtime where origin: Origin {
		auction_manager::AuctionManagerModule,
	}
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		auction_manager<T>,
		orml_tokens<T>,
		orml_auction<T>,
		cdp_treasury<T>,
		module_dex<T>,
	}
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
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}
pub type System = frame_system::Module<Runtime>;

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
}
pub type Tokens = orml_tokens::Module<Runtime>;

impl orml_auction::Config for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type AuctionId = AuctionId;
	type Handler = AuctionManagerModule;
	type WeightInfo = ();
}
pub type AuctionModule = orml_auction::Module<Runtime>;

ord_parameter_types! {
	pub const One: AccountId = 1;
}

parameter_types! {
	pub const GetStableCurrencyId: CurrencyId = AUSD;
	pub const MaxAuctionsCount: u32 = 10_000;
	pub const CDPTreasuryModuleId: ModuleId = ModuleId(*b"aca/cdpt");
}

impl cdp_treasury::Config for Runtime {
	type Event = TestEvent;
	type Currency = Tokens;
	type GetStableCurrencyId = GetStableCurrencyId;
	type AuctionManagerHandler = AuctionManagerModule;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
	type DEX = DEXModule;
	type MaxAuctionsCount = MaxAuctionsCount;
	type ModuleId = CDPTreasuryModuleId;
	type WeightInfo = ();
}
pub type CDPTreasuryModule = cdp_treasury::Module<Runtime>;

thread_local! {
	static RELATIVE_PRICE: RefCell<Option<Price>> = RefCell::new(Some(Price::one()));
}

pub struct MockPriceSource;
impl MockPriceSource {
	pub fn set_relative_price(price: Option<Price>) {
		RELATIVE_PRICE.with(|v| *v.borrow_mut() = price);
	}
}
impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_relative_price(_base: CurrencyId, _quota: CurrencyId) -> Option<Price> {
		RELATIVE_PRICE.with(|v| *v.borrow_mut())
	}

	fn get_price(_currency_id: CurrencyId) -> Option<Price> {
		Some(Price::one())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

parameter_types! {
	pub const DEXModuleId: ModuleId = ModuleId(*b"aca/dexm");
	pub const GetExchangeFee: (u32, u32) = (0, 100);
	pub const TradingPathLimit: usize = 3;
	pub EnabledTradingPairs : Vec<TradingPair> = vec![TradingPair::new(AUSD, BTC)];
}

impl module_dex::Config for Runtime {
	type Event = TestEvent;
	type Currency = Tokens;
	type GetExchangeFee = GetExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type ModuleId = DEXModuleId;
	type DEXIncentives = ();
	type WeightInfo = ();
	type ListingOrigin = EnsureSignedBy<One, AccountId>;
}
pub type DEXModule = module_dex::Module<Runtime>;

thread_local! {
	static IS_SHUTDOWN: RefCell<bool> = RefCell::new(false);
}

pub fn mock_shutdown() {
	IS_SHUTDOWN.with(|v| *v.borrow_mut() = true)
}

pub struct MockEmergencyShutdown;
impl EmergencyShutdown for MockEmergencyShutdown {
	fn is_shutdown() -> bool {
		IS_SHUTDOWN.with(|v| *v.borrow_mut())
	}
}

parameter_types! {
	pub MinimumIncrementSize: Rate = Rate::saturating_from_rational(1, 20);
	pub const AuctionTimeToClose: u64 = 100;
	pub const AuctionDurationSoftCap: u64 = 2000;
	pub const GetNativeCurrencyId: CurrencyId = ACA;
	pub const UnsignedPriority: u64 = 1 << 20;
}

impl Config for Runtime {
	type Event = TestEvent;
	type Currency = Tokens;
	type Auction = AuctionModule;
	type MinimumIncrementSize = MinimumIncrementSize;
	type AuctionTimeToClose = AuctionTimeToClose;
	type AuctionDurationSoftCap = AuctionDurationSoftCap;
	type GetStableCurrencyId = GetStableCurrencyId;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type CDPTreasury = CDPTreasuryModule;
	type DEX = DEXModule;
	type PriceSource = MockPriceSource;
	type UnsignedPriority = UnsignedPriority;
	type EmergencyShutdown = MockEmergencyShutdown;
	type WeightInfo = ();
}
pub type AuctionManagerModule = Module<Runtime>;

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				(ALICE, AUSD, 1000),
				(BOB, AUSD, 1000),
				(CAROL, AUSD, 1000),
				(ALICE, BTC, 1000),
				(BOB, BTC, 1000),
				(CAROL, BTC, 1000),
				(ALICE, ACA, 1000),
				(BOB, ACA, 1000),
				(CAROL, ACA, 1000),
			],
		}
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		module_dex::GenesisConfig::<Runtime> {
			initial_listing_trading_pairs: vec![],
			initial_enabled_trading_pairs: EnabledTradingPairs::get(),
			initial_added_liquidity_pools: vec![],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
