//! # CDP Treasury Module
//!
//! ## Overview
//!
//! CDP Treasury manages the accumulated interest and bad debts generated by CDPs,
//! and handle excessive surplus or debits timely in order to keep the system healthy with low risk.
//! It's the only entry for issuing/burning stable coin for whole system.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{EnsureOrigin, Get},
};
use frame_system::{self as system, ensure_root};
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use primitives::{Balance, CurrencyId};
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	DispatchError, DispatchResult, FixedPointNumber, ModuleId,
};
use support::{AuctionManager, CDPTreasury, CDPTreasuryExtended, DEXManager, OnEmergencyShutdown, Ratio};

mod benchmarking;
mod mock;
mod tests;

const MODULE_ID: ModuleId = ModuleId(*b"aca/cdpt");

pub trait Trait: system::Trait {
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;

	/// The origin which may update parameters. Root can always do this.
	type UpdateOrigin: EnsureOrigin<Self::Origin>;

	/// The Currency for issuing/burning stable coin
	type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;

	/// Stablecoin currency id
	type GetStableCurrencyId: Get<CurrencyId>;

	/// Auction manager creates different types of auction to handle system surplus and debit, and confiscated collateral assets
	type AuctionManagerHandler: AuctionManager<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;

	/// Dex manager is used to swap confiscated collateral assets to stable coin
	type DEX: DEXManager<Self::AccountId, CurrencyId, Balance>;

	/// the cap of lots number when create collateral auction on a liquidation
	/// or to create debit/surplus auction on block end.
	/// If set to 0, does not work.
	type MaxAuctionsCount: Get<u32>;
}

decl_event!(
	pub enum Event {
		/// The fixed size for surplus auction updated (new_size)
		SurplusAuctionFixedSizeUpdated(Balance),
		/// The buffer size of surplus pool updated (new_size)
		SurplusBufferSizeUpdated(Balance),
		/// The initial supply amount of a debit auction updated (new_amount)
		InitialAmountPerDebitAuctionUpdated(Balance),
		/// The fixed size for debit auction updated (new_size)
		DebitAuctionFixedSizeUpdated(Balance),
		/// The fixed size for collateral auction under specific collateral type updated (collateral_type, new_size)
		CollateralAuctionMaximumSizeUpdated(CurrencyId, Balance),
	}
);

decl_error! {
	/// Error for cdp treasury module.
	pub enum Error for Module<T: Trait> {
		/// The collateral amount of CDP treasury is not enough
		CollateralNotEnough,
		/// Collateral Amount overflow
		CollateralOverflow,
		/// The amount of surplus pool is not enough
		SurplusPoolNotEnough,
		/// Surplus pool overflow
		SurplusPoolOverflow,
		/// debit pool overflow
		DebitPoolOverflow,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as CDPTreasury {
		/// The fixed amount of stable coin for sale per surplus auction
		pub SurplusAuctionFixedSize get(fn surplus_auction_fixed_size) config(): Balance;

		/// The buffer size of surplus pool, the system will process the surplus through
		/// surplus auction when above this value
		pub SurplusBufferSize get(fn surplus_buffer_size) config(): Balance;

		/// Initial amount of native token for sale per debit auction
		pub InitialAmountPerDebitAuction get(fn initial_amount_per_debit_auction) config(): Balance;

		/// The fixed amount of stable coin per surplus auction wants to get
		pub DebitAuctionFixedSize get(fn debit_auction_fixed_size) config(): Balance;

		/// The maximum amount of collateral amount for sale per collateral auction
		pub CollateralAuctionMaximumSize get(fn collateral_auction_maximum_size): map hasher(twox_64_concat) CurrencyId => Balance;

		/// Current total debit value of system. It's not same as debit in CDP engine,
		/// it is the bad debt of the system.
		pub DebitPool get(fn debit_pool): Balance;

		/// Current total surplus of system.
		pub SurplusPool get(fn surplus_pool): Balance;

		/// Mapping from collateral type to collateral assets amount kept in CDP treasury
		pub TotalCollaterals get(fn total_collaterals): map hasher(twox_64_concat) CurrencyId => Balance;

		/// System shutdown flag
		pub IsShutdown get(fn is_shutdown): bool;
	}

	add_extra_genesis {
		config(collateral_auction_maximum_size): Vec<(CurrencyId, Balance)>;

		build(|config: &GenesisConfig| {
			config.collateral_auction_maximum_size.iter().for_each(|(currency_id, size)| {
				CollateralAuctionMaximumSize::insert(currency_id, size);
			})
		})
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		/// Stablecoin currency id
		const GetStableCurrencyId: CurrencyId = T::GetStableCurrencyId::get();

		/// Lots cap when create auction
		const MaxAuctionsCount: u32 = T::MaxAuctionsCount::get();

		/// Update parameters related to surplus and debit auction
		///
		/// The dispatch origin of this call must be `UpdateOrigin` or _Root_.
		///
		/// - `surplus_auction_fixed_size`: new fixed amount of stable coin for sale per surplus auction, `None` means do not update
		/// - `surplus_buffer_size`: new buffer size of surplus pool, `None` means do not update
		/// - `initial_amount_per_debit_auction`: initial amount of native token for sale per debit auction, `None` means do not update
		/// - `debit_auction_fixed_size`: the fixed amount of stable coin per collateral auction wants to get, `None` means do not update
		///
		/// # <weight>
		/// - Complexity: `O(1)`
		/// - Db reads:
		/// - Db writes: `SurplusAuctionFixedSize`, `SurplusBufferSize`, `InitialAmountPerDebitAuction`, `DebitAuctionFixedSize`
		/// -------------------
		/// Base Weight: 20.18 µs
		/// # </weight>
		#[weight = 20_000_000 + T::DbWeight::get().reads_writes(0, 4)]
		pub fn set_debit_and_surplus_handle_params(
			origin,
			surplus_auction_fixed_size: Option<Balance>,
			surplus_buffer_size: Option<Balance>,
			initial_amount_per_debit_auction: Option<Balance>,
			debit_auction_fixed_size: Option<Balance>,
		) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			if let Some(amount) = surplus_auction_fixed_size {
				SurplusAuctionFixedSize::put(amount);
				Self::deposit_event(Event::SurplusAuctionFixedSizeUpdated(amount));
			}
			if let Some(amount) = surplus_buffer_size {
				SurplusBufferSize::put(amount);
				Self::deposit_event(Event::SurplusBufferSizeUpdated(amount));
			}
			if let Some(amount) = initial_amount_per_debit_auction {
				InitialAmountPerDebitAuction::put(amount);
				Self::deposit_event(Event::InitialAmountPerDebitAuctionUpdated(amount));
			}
			if let Some(amount) = debit_auction_fixed_size {
				DebitAuctionFixedSize::put(amount);
				Self::deposit_event(Event::DebitAuctionFixedSizeUpdated(amount));
			}
		}

		/// Update parameters related to collateral auction under specific collateral type
		///
		/// The dispatch origin of this call must be `UpdateOrigin` or _Root_.
		///
		/// - `currency_id`: collateral type
		/// - `surplus_buffer_size`: collateral auction maximum size
		///
		/// # <weight>
		/// - Complexity: `O(1)`
		/// - Db reads:
		/// - Db writes: `CollateralAuctionMaximumSize`
		/// -------------------
		/// Base Weight: 15.59 µs
		/// # </weight>
		#[weight = 16_000_000 + T::DbWeight::get().reads_writes(0, 1)]
		pub fn set_collateral_auction_maximum_size(origin, currency_id: CurrencyId, size: Balance) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			CollateralAuctionMaximumSize::insert(currency_id, size);
			Self::deposit_event(Event::CollateralAuctionMaximumSizeUpdated(currency_id, size));
		}

		/// Handle excessive surplus or debits of system when block end
		fn on_finalize(_now: T::BlockNumber) {
			// offset the same amount between debit pool and surplus pool
			Self::offset_surplus_and_debit();

			// Stop to create surplus auction and debit auction after emergency shutdown happend.
			if !Self::is_shutdown() {
				let max_auctions_count: u32 = T::MaxAuctionsCount::get();
				let mut created_lots: u32 = 0;

				let surplus_auction_fixed_size = Self::surplus_auction_fixed_size();
				if !surplus_auction_fixed_size.is_zero() {
					let mut remain_surplus_pool = Self::surplus_pool();
					let surplus_buffer_size = Self::surplus_buffer_size();
					let total_surplus_in_auction = T::AuctionManagerHandler::get_total_surplus_in_auction();

					// create surplus auction requires:
					// surplus_pool >= total_surplus_in_auction + surplus_buffer_size + surplus_auction_fixed_size
					while remain_surplus_pool >= total_surplus_in_auction + surplus_buffer_size + surplus_auction_fixed_size {
						if max_auctions_count != 0 && created_lots >= max_auctions_count {
							break
						}
						T::AuctionManagerHandler::new_surplus_auction(surplus_auction_fixed_size);
						created_lots += 1;
						remain_surplus_pool -= surplus_auction_fixed_size;
					}
				}

				let debit_auction_fixed_size = Self::debit_auction_fixed_size();
				let initial_amount_per_debit_auction = Self::initial_amount_per_debit_auction();
				if !debit_auction_fixed_size.is_zero() && !initial_amount_per_debit_auction.is_zero() {
					let mut remain_debit_pool = Self::debit_pool();
					let total_debit_in_auction = T::AuctionManagerHandler::get_total_debit_in_auction();
					let total_target_in_auction = T::AuctionManagerHandler::get_total_target_in_auction();

					// create debit auction requires:
					// debit_pool > total_debit_in_auction + total_target_in_auction + debit_auction_fixed_size
					while remain_debit_pool >= total_debit_in_auction + total_target_in_auction + debit_auction_fixed_size {
						if max_auctions_count != 0 && created_lots >= max_auctions_count {
							break
						}
						T::AuctionManagerHandler::new_debit_auction(initial_amount_per_debit_auction, debit_auction_fixed_size);
						created_lots += 1;
						remain_debit_pool -= debit_auction_fixed_size;
					}
				}
			}
		}
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn offset_surplus_and_debit() {
		let offset_amount = sp_std::cmp::min(Self::debit_pool(), Self::surplus_pool());

		// burn the amount that is equal to offset amount of stable coin.
		if !offset_amount.is_zero()
			&& T::Currency::withdraw(T::GetStableCurrencyId::get(), &Self::account_id(), offset_amount).is_ok()
		{
			DebitPool::mutate(|debit| *debit -= offset_amount);
			SurplusPool::mutate(|surplus| *surplus -= offset_amount);
		}
	}

	pub fn emergency_shutdown() {
		<IsShutdown>::put(true);
	}
}

impl<T: Trait> CDPTreasury<T::AccountId> for Module<T> {
	type Balance = Balance;
	type CurrencyId = CurrencyId;

	fn get_surplus_pool() -> Self::Balance {
		Self::surplus_pool()
	}

	fn get_debit_pool() -> Self::Balance {
		Self::debit_pool()
	}

	fn get_total_collaterals(id: Self::CurrencyId) -> Self::Balance {
		Self::total_collaterals(id)
	}

	fn on_system_debit(amount: Self::Balance) -> DispatchResult {
		let new_debit_pool = Self::debit_pool()
			.checked_add(amount)
			.ok_or(Error::<T>::DebitPoolOverflow)?;
		DebitPool::put(new_debit_pool);
		Ok(())
	}

	fn on_system_surplus(amount: Self::Balance) -> DispatchResult {
		let new_surplus_pool = Self::surplus_pool()
			.checked_add(amount)
			.ok_or(Error::<T>::SurplusPoolOverflow)?;
		T::Currency::deposit(T::GetStableCurrencyId::get(), &Self::account_id(), amount)?;
		SurplusPool::put(new_surplus_pool);
		Ok(())
	}

	fn deposit_backed_debit_to(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		T::Currency::deposit(T::GetStableCurrencyId::get(), who, amount)
	}

	fn deposit_unbacked_debit_to(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		Self::on_system_debit(amount)?;
		T::Currency::deposit(T::GetStableCurrencyId::get(), who, amount)
	}

	fn withdraw_backed_debit_from(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		T::Currency::withdraw(T::GetStableCurrencyId::get(), who, amount)
	}

	fn transfer_surplus_from(from: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		let new_surplus_pool = Self::surplus_pool()
			.checked_add(amount)
			.ok_or(Error::<T>::SurplusPoolOverflow)?;
		T::Currency::transfer(T::GetStableCurrencyId::get(), from, &Self::account_id(), amount)?;
		SurplusPool::put(new_surplus_pool);
		Ok(())
	}

	fn transfer_collateral_to(
		currency_id: Self::CurrencyId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		let new_total_collateral = Self::total_collaterals(currency_id)
			.checked_sub(amount)
			.ok_or(Error::<T>::CollateralNotEnough)?;
		T::Currency::transfer(currency_id, &Self::account_id(), to, amount)?;
		TotalCollaterals::insert(currency_id, new_total_collateral);
		Ok(())
	}

	fn transfer_collateral_from(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		let new_total_collateral = Self::total_collaterals(currency_id)
			.checked_add(amount)
			.ok_or(Error::<T>::CollateralOverflow)?;
		T::Currency::transfer(currency_id, from, &Self::account_id(), amount)?;
		TotalCollaterals::insert(currency_id, new_total_collateral);
		Ok(())
	}

	fn get_debit_proportion(amount: Self::Balance) -> Ratio {
		let stable_total_supply = T::Currency::total_issuance(T::GetStableCurrencyId::get());
		Ratio::checked_from_rational(amount, stable_total_supply).unwrap_or_default()
	}
}

impl<T: Trait> CDPTreasuryExtended<T::AccountId> for Module<T> {
	fn swap_collateral_to_stable(
		currency_id: CurrencyId,
		supply_amount: Balance,
		target_amount: Balance,
	) -> sp_std::result::Result<Balance, DispatchError> {
		ensure!(
			Self::total_collaterals(currency_id) >= supply_amount,
			Error::<T>::CollateralNotEnough,
		);
		T::Currency::ensure_can_withdraw(currency_id, &Self::account_id(), supply_amount)?;

		let amount = T::DEX::exchange_currency(
			Self::account_id(),
			currency_id,
			supply_amount,
			T::GetStableCurrencyId::get(),
			target_amount,
		)?;

		TotalCollaterals::mutate(currency_id, |balance| *balance -= supply_amount);
		SurplusPool::mutate(|surplus| *surplus += amount);

		Ok(amount)
	}

	fn create_collateral_auctions(
		currency_id: CurrencyId,
		amount: Balance,
		target: Balance,
		refund_receiver: T::AccountId,
	) {
		if Self::total_collaterals(currency_id)
			>= amount + T::AuctionManagerHandler::get_total_collateral_in_auction(currency_id)
		{
			let collateral_auction_maximum_size = Self::collateral_auction_maximum_size(currency_id);
			let mut unhandled_collateral_amount = amount;
			let mut unhandled_target = target;
			let max_auctions_count: u32 = T::MaxAuctionsCount::get();
			let mut created_lots: u32 = 0;

			while !unhandled_collateral_amount.is_zero() {
				let (lot_collateral_amount, lot_target) = if unhandled_collateral_amount
					> collateral_auction_maximum_size
					&& !collateral_auction_maximum_size.is_zero()
				{
					if max_auctions_count != 0 && created_lots >= max_auctions_count {
						(unhandled_collateral_amount, unhandled_target)
					} else {
						created_lots += 1;
						let proportion =
							Ratio::checked_from_rational(collateral_auction_maximum_size, amount).unwrap_or_default();
						(collateral_auction_maximum_size, proportion.saturating_mul_int(target))
					}
				} else {
					(unhandled_collateral_amount, unhandled_target)
				};

				T::AuctionManagerHandler::new_collateral_auction(
					&refund_receiver,
					currency_id,
					lot_collateral_amount,
					lot_target,
				);

				unhandled_collateral_amount -= lot_collateral_amount;
				unhandled_target -= lot_target;
			}
		}
	}
}

impl<T: Trait> OnEmergencyShutdown for Module<T> {
	fn on_emergency_shutdown() {
		Self::emergency_shutdown();
	}
}
