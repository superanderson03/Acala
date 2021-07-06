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

//! # DEX Module
//!
//! ## Overview
//!
//! Built-in decentralized exchange modules in Acala network, the swap
//! mechanism refers to the design of Uniswap V2. In addition to being used for
//! trading, DEX also participates in CDP liquidation, which is faster than
//! liquidation by auction when the liquidity is sufficient. And providing
//! market making liquidity for DEX will also receive stable currency as
//! additional reward for its participation in the CDP liquidation.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::unused_unit)]
#![allow(clippy::collapsible_if)]

use frame_support::{log, pallet_prelude::*, traits::MaxEncodedLen, transactional, PalletId};
use frame_system::pallet_prelude::*;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use primitives::{Balance, CurrencyId, TradingPair};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{AccountIdConversion, One, UniqueSaturatedInto, Zero},
	DispatchError, DispatchResult, FixedPointNumber, RuntimeDebug, SaturatedConversion,
};
use sp_std::{convert::TryInto, prelude::*, vec};
use support::{CurrencyIdMapping, DEXIncentives, DEXManager, ExchangeRate, Ratio};

mod mock;
mod tests;
pub mod weights;

pub use module::*;
pub use weights::WeightInfo;

/// Parameters of TradingPair in Provisioning status
#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq, MaxEncodedLen)]
pub struct ProvisioningParameters<Balance, BlockNumber> {
	/// limit contribution per time.
	min_contribution: (Balance, Balance),
	/// target provision that trading pair could to be Enabled.
	target_provision: (Balance, Balance),
	/// accumulated provision amount for this Provisioning trading pair.
	accumulated_provision: (Balance, Balance),
	/// The number of block that status can be converted to Enabled.
	not_before: BlockNumber,
}

/// Status for TradingPair
#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, PartialEq, Eq, MaxEncodedLen)]
pub enum TradingPairStatus<Balance, BlockNumber> {
	/// Default status,
	/// can withdraw liquidity, re-enable and list this trading pair.
	Disabled,
	/// TradingPair is Provisioning,
	/// can add provision and disable this trading pair.
	Provisioning(ProvisioningParameters<Balance, BlockNumber>),
	/// TradingPair is Enabled,
	/// can add/remove liquidity, trading and disable this trading pair.
	Enabled,
}

impl<Balance, BlockNumber> Default for TradingPairStatus<Balance, BlockNumber> {
	fn default() -> Self {
		Self::Disabled
	}
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Currency for transfer currencies
		type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;

		/// Trading fee rate
		/// The first item of the tuple is the numerator of the fee rate, second
		/// item is the denominator, fee_rate = numerator / denominator,
		/// use (u32, u32) over `Rate` type to minimize internal division
		/// operation.
		#[pallet::constant]
		type GetExchangeFee: Get<(u32, u32)>;

		/// The limit for length of trading path
		#[pallet::constant]
		type TradingPathLimit: Get<u32>;

		/// The DEX's module id, keep all assets in DEX.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Mapping between CurrencyId and ERC20 address so user can use Erc20
		/// address as LP token.
		type CurrencyIdMapping: CurrencyIdMapping;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// DEX incentives
		type DEXIncentives: DEXIncentives<Self::AccountId, CurrencyId, Balance>;

		/// The origin which may list, enable or disable trading pairs.
		type ListingOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Trading pair is already Enabled
		AlreadyEnabled,
		/// Trading pair must be in Enabled status
		MustBeEnabled,
		/// Trading pair must be in Provisioning status
		MustBeProvisioning,
		/// Trading pair must be in Disabled status
		MustBeDisabled,
		/// This trading pair is not allowed to be listed
		NotAllowedList,
		/// The increment of provision is invalid
		InvalidContributionIncrement,
		/// The increment of liquidity is invalid
		InvalidLiquidityIncrement,
		/// Invalid currency id
		InvalidCurrencyId,
		/// Invalid trading path length
		InvalidTradingPathLength,
		/// Target amount is less to min_target_amount
		InsufficientTargetAmount,
		/// Supply amount is more than max_supply_amount
		ExcessiveSupplyAmount,
		/// The swap will cause unacceptable price impact
		ExceedPriceImpactLimit,
		/// Liquidity is not enough
		InsufficientLiquidity,
		/// The supply amount is zero
		ZeroSupplyAmount,
		/// The target amount is zero
		ZeroTargetAmount,
		/// The share increment is unacceptable
		UnacceptableShareIncrement,
		/// The liquidity withdrawn is unacceptable
		UnacceptableLiquidityWithdrawn,
		/// The swap dosen't meet the invariant check
		InvariantCheckFailed,
		/// The Provision is unqualified to be converted to `Enabled`
		UnqualifiedProvision,
		/// Trading pair is still provisioning
		StillProvisioning,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// add provision success \[who, currency_id_0, contribution_0,
		/// currency_id_1, contribution_1\]
		AddProvision(T::AccountId, CurrencyId, Balance, CurrencyId, Balance),
		/// Add liquidity success. \[who, currency_id_0, pool_0_increment,
		/// currency_id_1, pool_1_increment, share_increment\]
		AddLiquidity(T::AccountId, CurrencyId, Balance, CurrencyId, Balance, Balance),
		/// Remove liquidity from the trading pool success. \[who,
		/// currency_id_0, pool_0_decrement, currency_id_1, pool_1_decrement,
		/// share_decrement\]
		RemoveLiquidity(T::AccountId, CurrencyId, Balance, CurrencyId, Balance, Balance),
		/// Use supply currency to swap target currency. \[trader, trading_path,
		/// supply_currency_amount, target_currency_amount\]
		Swap(T::AccountId, Vec<CurrencyId>, Balance, Balance),
		/// Enable trading pair. \[trading_pair\]
		EnableTradingPair(TradingPair),
		/// List provisioning trading pair. \[trading_pair\]
		ListProvisioning(TradingPair),
		/// Disable trading pair. \[trading_pair\]
		DisableTradingPair(TradingPair),
		/// Provisioning trading pair convert to Enabled. \[trading_pair,
		/// pool_0_amount, pool_1_amount, total_share_amount\]
		ProvisioningToEnabled(TradingPair, Balance, Balance, Balance),
	}

	/// Liquidity pool for TradingPair.
	///
	/// LiquidityPool: map TradingPair => (Balance, Balance)
	#[pallet::storage]
	#[pallet::getter(fn liquidity_pool)]
	pub type LiquidityPool<T: Config> = StorageMap<_, Twox64Concat, TradingPair, (Balance, Balance), ValueQuery>;

	/// Status for TradingPair.
	///
	/// TradingPairStatuses: map TradingPair => TradingPairStatus
	#[pallet::storage]
	#[pallet::getter(fn trading_pair_statuses)]
	pub type TradingPairStatuses<T: Config> =
		StorageMap<_, Twox64Concat, TradingPair, TradingPairStatus<Balance, T::BlockNumber>, ValueQuery>;

	/// Provision of TradingPair by AccountId.
	///
	/// ProvisioningPool: double_map TradingPair, AccountId => (Balance,
	/// Balance)
	#[pallet::storage]
	#[pallet::getter(fn provisioning_pool)]
	pub type ProvisioningPool<T: Config> =
		StorageDoubleMap<_, Twox64Concat, TradingPair, Twox64Concat, T::AccountId, (Balance, Balance), ValueQuery>;

	/// Initial exchange rate, used to calculate the dex share amount for founders of provisioning
	///
	/// InitialShareExchangeRates: map TradingPair => (ExchangeRate, ExchangeRate)
	#[pallet::storage]
	#[pallet::getter(fn initial_share_exchange_rates)]
	pub type InitialShareExchangeRates<T: Config> =
		StorageMap<_, Twox64Concat, TradingPair, (ExchangeRate, ExchangeRate), ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_listing_trading_pairs: Vec<(TradingPair, (Balance, Balance), (Balance, Balance), T::BlockNumber)>,
		pub initial_enabled_trading_pairs: Vec<TradingPair>,
		pub initial_added_liquidity_pools: Vec<(T::AccountId, Vec<(TradingPair, (Balance, Balance))>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				initial_listing_trading_pairs: vec![],
				initial_enabled_trading_pairs: vec![],
				initial_added_liquidity_pools: vec![],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.initial_listing_trading_pairs.iter().for_each(
				|(trading_pair, min_contribution, target_provision, not_before)| {
					TradingPairStatuses::<T>::insert(
						trading_pair,
						TradingPairStatus::Provisioning(ProvisioningParameters {
							min_contribution: *min_contribution,
							target_provision: *target_provision,
							accumulated_provision: Default::default(),
							not_before: *not_before,
						}),
					);
				},
			);

			self.initial_enabled_trading_pairs.iter().for_each(|trading_pair| {
				TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::<_, _>::Enabled);
			});

			self.initial_added_liquidity_pools
				.iter()
				.for_each(|(who, trading_pairs_data)| {
					trading_pairs_data
						.iter()
						.for_each(|(trading_pair, (deposit_amount_0, deposit_amount_1))| {
							let result = match <Pallet<T>>::trading_pair_statuses(trading_pair) {
								TradingPairStatus::<_, _>::Enabled => <Pallet<T>>::do_add_liquidity(
									&who,
									trading_pair.first(),
									trading_pair.second(),
									*deposit_amount_0,
									*deposit_amount_1,
									Default::default(),
									false,
								),
								_ => Err(Error::<T>::MustBeEnabled.into()),
							};

							assert!(result.is_ok(), "genesis add lidquidity pool failed.");
						});
				});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Trading with DEX, swap with exact supply amount
		///
		/// - `path`: trading path.
		/// - `supply_amount`: exact supply amount.
		/// - `min_target_amount`: acceptable minimum target amount.
		#[pallet::weight(<T as Config>::WeightInfo::swap_with_exact_supply(path.len() as u32))]
		#[transactional]
		pub fn swap_with_exact_supply(
			origin: OriginFor<T>,
			path: Vec<CurrencyId>,
			#[pallet::compact] supply_amount: Balance,
			#[pallet::compact] min_target_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_swap_with_exact_supply(&who, &path, supply_amount, min_target_amount, None)?;
			Ok(().into())
		}

		/// Trading with DEX, swap with exact target amount
		///
		/// - `path`: trading path.
		/// - `target_amount`: exact target amount.
		/// - `max_supply_amount`: acceptable maximum supply amount.
		#[pallet::weight(<T as Config>::WeightInfo::swap_with_exact_target(path.len() as u32))]
		#[transactional]
		pub fn swap_with_exact_target(
			origin: OriginFor<T>,
			path: Vec<CurrencyId>,
			#[pallet::compact] target_amount: Balance,
			#[pallet::compact] max_supply_amount: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_swap_with_exact_target(&who, &path, target_amount, max_supply_amount, None)?;
			Ok(().into())
		}

		/// Add liquidity to Enabled trading pair.
		/// - Add provision success will record the provision, issue shares to caller in the initial
		///   exchange rate when trading pair convert to Enabled.
		///
		/// - `currency_id_a`: currency id A.
		/// - `currency_id_b`: currency id B.
		/// - `max_amount_a`: maximum amount of currency_id_a is allowed to inject to liquidity
		///   pool.
		/// - `max_amount_b`: maximum amount of currency_id_b is allowed to inject to liquidity
		///   pool.
		/// - `min_share_increment`: minimum acceptable share amount.
		/// - `stake_increment_share`: indicates whether to stake increased dex share to earn
		///   incentives
		#[pallet::weight(if *stake_increment_share {
			<T as Config>::WeightInfo::add_liquidity_and_stake()
		} else {
			<T as Config>::WeightInfo::add_liquidity()
		})]
		#[transactional]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			#[pallet::compact] max_amount_a: Balance,
			#[pallet::compact] max_amount_b: Balance,
			#[pallet::compact] min_share_increment: Balance,
			stake_increment_share: bool,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_add_liquidity(
				&who,
				currency_id_a,
				currency_id_b,
				max_amount_a,
				max_amount_b,
				min_share_increment,
				stake_increment_share,
			)?;
			Ok(().into())
		}

		/// Add provision to Provisioning trading pair.
		/// If succecced, will record the provision, but shares issuing will happen after the
		/// trading pair convert to Enabled status.
		///
		/// - `currency_id_a`: currency id A.
		/// - `currency_id_b`: currency id B.
		/// - `amount_a`: provision amount for currency_id_a.
		/// - `amount_b`: provision amount for currency_id_b.
		#[pallet::weight(<T as Config>::WeightInfo::add_provision())]
		#[transactional]
		pub fn add_provision(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			#[pallet::compact] amount_a: Balance,
			#[pallet::compact] amount_b: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_add_provision(&who, currency_id_a, currency_id_b, amount_a, amount_b)?;
			Ok(().into())
		}

		/// Claim dex share for founders who have participated in trading pair provision.
		///
		/// - `currency_id_a`: currency id A.
		/// - `currency_id_b`: currency id B.
		#[pallet::weight(<T as Config>::WeightInfo::claim_dex_share())]
		#[transactional]
		pub fn claim_dex_share(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_claim_dex_share(&who, currency_id_a, currency_id_b)?;
			Ok(().into())
		}

		/// Remove liquidity from specific liquidity pool in the form of burning
		/// shares, and withdrawing currencies in trading pairs from liquidity
		/// pool in proportion, and withdraw liquidity incentive interest.
		///
		/// - `currency_id_a`: currency id A.
		/// - `currency_id_b`: currency id B.
		/// - `remove_share`: liquidity amount to remove.
		/// - `min_withdrawn_a`: minimum acceptable withrawn for currency_id_a.
		/// - `min_withdrawn_b`: minimum acceptable withrawn for currency_id_b.
		/// - `by_unstake`: this flag indicates whether to withdraw share which is on incentives.
		#[pallet::weight(if *by_unstake {
			<T as Config>::WeightInfo::remove_liquidity_by_unstake()
		} else {
			<T as Config>::WeightInfo::remove_liquidity()
		})]
		#[transactional]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			#[pallet::compact] remove_share: Balance,
			#[pallet::compact] min_withdrawn_a: Balance,
			#[pallet::compact] min_withdrawn_b: Balance,
			by_unstake: bool,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_remove_liquidity(
				&who,
				currency_id_a,
				currency_id_b,
				remove_share,
				min_withdrawn_a,
				min_withdrawn_b,
				by_unstake,
			)?;
			Ok(().into())
		}

		/// List a new provisioning trading pair.
		#[pallet::weight((<T as Config>::WeightInfo::list_provisioning(), DispatchClass::Operational))]
		#[transactional]
		pub fn list_provisioning(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			min_contribution_a: Balance,
			min_contribution_b: Balance,
			target_provision_a: Balance,
			target_provision_b: Balance,
			not_before: T::BlockNumber,
		) -> DispatchResultWithPostInfo {
			T::ListingOrigin::ensure_origin(origin)?;

			let trading_pair =
				TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
			ensure!(
				matches!(
					Self::trading_pair_statuses(trading_pair),
					TradingPairStatus::<_, _>::Disabled
				),
				Error::<T>::MustBeDisabled
			);
			ensure!(
				T::Currency::total_issuance(trading_pair.dex_share_currency_id()).is_zero(),
				Error::<T>::NotAllowedList
			);

			if let CurrencyId::Erc20(address) = currency_id_a {
				T::CurrencyIdMapping::set_erc20_mapping(address)?;
			}
			if let CurrencyId::Erc20(address) = currency_id_b {
				T::CurrencyIdMapping::set_erc20_mapping(address)?;
			}

			let (min_contribution, target_provision) = if currency_id_a == trading_pair.first() {
				(
					(min_contribution_a, min_contribution_b),
					(target_provision_a, target_provision_b),
				)
			} else {
				(
					(min_contribution_b, min_contribution_a),
					(target_provision_b, target_provision_a),
				)
			};

			TradingPairStatuses::<T>::insert(
				trading_pair,
				TradingPairStatus::Provisioning(ProvisioningParameters {
					min_contribution,
					target_provision,
					accumulated_provision: Default::default(),
					not_before,
				}),
			);
			Self::deposit_event(Event::ListProvisioning(trading_pair));
			Ok(().into())
		}

		/// List a new trading pair, trading pair will become Enabled status
		/// after provision process.
		#[pallet::weight((<T as Config>::WeightInfo::update_provisioning_parameters(), DispatchClass::Operational))]
		#[transactional]
		pub fn update_provisioning_parameters(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			min_contribution_a: Balance,
			min_contribution_b: Balance,
			target_provision_a: Balance,
			target_provision_b: Balance,
			not_before: T::BlockNumber,
		) -> DispatchResultWithPostInfo {
			T::ListingOrigin::ensure_origin(origin)?;
			let trading_pair =
				TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;

			match Self::trading_pair_statuses(trading_pair) {
				TradingPairStatus::Provisioning(provisioning_parameters) => {
					let (min_contribution, target_provision) = if currency_id_a == trading_pair.first() {
						(
							(min_contribution_a, min_contribution_b),
							(target_provision_a, target_provision_b),
						)
					} else {
						(
							(min_contribution_b, min_contribution_a),
							(target_provision_b, target_provision_a),
						)
					};
					TradingPairStatuses::<T>::insert(
						trading_pair,
						TradingPairStatus::Provisioning(ProvisioningParameters {
							min_contribution,
							target_provision,
							accumulated_provision: provisioning_parameters.accumulated_provision,
							not_before,
						}),
					);
				}
				_ => return Err(Error::<T>::MustBeProvisioning.into()),
			}

			Ok(().into())
		}

		/// Enable a Provisioning trading pair if meet the condition.
		#[pallet::weight((<T as Config>::WeightInfo::end_provisioning(), DispatchClass::Operational))]
		#[transactional]
		pub fn end_provisioning(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;

			let trading_pair =
				TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;

			match Self::trading_pair_statuses(trading_pair) {
				TradingPairStatus::<_, _>::Provisioning(provisioning_parameters) => {
					let (total_provision_0, total_provision_1) = provisioning_parameters.accumulated_provision;
					ensure!(
						frame_system::Pallet::<T>::block_number() >= provisioning_parameters.not_before
							&& !total_provision_0.is_zero()
							&& !total_provision_1.is_zero()
							&& (total_provision_0 >= provisioning_parameters.target_provision.0
								|| total_provision_1 >= provisioning_parameters.target_provision.1),
						Error::<T>::UnqualifiedProvision
					);

					// end provisioning and convert trading pair to `Enabled`
					let (share_exchange_rate_0, share_exchange_rate_1) = if total_provision_0 > total_provision_1 {
						(
							ExchangeRate::one(),
							ExchangeRate::checked_from_rational(total_provision_0, total_provision_1)
								.unwrap_or_default(),
						)
					} else {
						(
							ExchangeRate::checked_from_rational(total_provision_1, total_provision_0)
								.unwrap_or_default(),
							ExchangeRate::one(),
						)
					};
					let total_shares_to_issue = share_exchange_rate_0
						.saturating_mul_int(total_provision_0)
						.saturating_add(share_exchange_rate_1.saturating_mul_int(total_provision_1));

					// issue total shares to module account
					T::Currency::deposit(
						trading_pair.dex_share_currency_id(),
						&Self::account_id(),
						total_shares_to_issue,
					)?;

					// inject provision to liquidity pool
					LiquidityPool::<T>::mutate(trading_pair, |(pool_0, pool_1)| {
						*pool_0 = pool_0.saturating_add(total_provision_0);
						*pool_1 = pool_1.saturating_add(total_provision_1);
					});

					// update trading_pair to Enabled status
					TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::<_, _>::Enabled);

					// record initial exchange rate so that founders can use it to calculate their own shares
					InitialShareExchangeRates::<T>::insert(
						trading_pair,
						(share_exchange_rate_0, share_exchange_rate_1),
					);

					Self::deposit_event(Event::ProvisioningToEnabled(
						trading_pair,
						total_provision_0,
						total_provision_1,
						total_shares_to_issue,
					));
				}
				_ => return Err(Error::<T>::MustBeProvisioning.into()),
			}

			Ok(().into())
		}

		/// Enable a trading pair
		/// if the status of trading pair is `Disabled`, or `Provisioning` without any accumulated
		/// provision, enable it directly.
		#[pallet::weight((<T as Config>::WeightInfo::enable_trading_pair(), DispatchClass::Operational))]
		#[transactional]
		pub fn enable_trading_pair(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
		) -> DispatchResultWithPostInfo {
			T::ListingOrigin::ensure_origin(origin)?;
			let trading_pair =
				TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
			match Self::trading_pair_statuses(trading_pair) {
				TradingPairStatus::<_, _>::Disabled => {}
				TradingPairStatus::<_, _>::Provisioning(provisioning_parameters) => {
					ensure!(
						provisioning_parameters.accumulated_provision.0.is_zero()
							&& provisioning_parameters.accumulated_provision.1.is_zero(),
						Error::<T>::StillProvisioning
					);
				}
				TradingPairStatus::<_, _>::Enabled => return Err(Error::<T>::AlreadyEnabled.into()),
			}

			TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::Enabled);
			Self::deposit_event(Event::EnableTradingPair(trading_pair));
			Ok(().into())
		}

		/// Disable a `Enabled` trading pair.
		#[pallet::weight((<T as Config>::WeightInfo::disable_trading_pair(), DispatchClass::Operational))]
		#[transactional]
		pub fn disable_trading_pair(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
		) -> DispatchResultWithPostInfo {
			T::ListingOrigin::ensure_origin(origin)?;
			let trading_pair =
				TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
			ensure!(
				matches!(
					Self::trading_pair_statuses(trading_pair),
					TradingPairStatus::<_, _>::Enabled
				),
				Error::<T>::MustBeEnabled
			);

			TradingPairStatuses::<T>::insert(trading_pair, TradingPairStatus::Disabled);
			Self::deposit_event(Event::DisableTradingPair(trading_pair));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	fn do_claim_dex_share(who: &T::AccountId, currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> DispatchResult {
		let trading_pair =
			TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
		ensure!(
			!matches!(
				Self::trading_pair_statuses(trading_pair),
				TradingPairStatus::<_, _>::Provisioning(_)
			),
			Error::<T>::StillProvisioning
		);

		ProvisioningPool::<T>::try_mutate_exists(trading_pair, who, |maybe_contribution| -> DispatchResult {
			if let Some((contribution_0, contribution_1)) = maybe_contribution.take() {
				let (exchange_rate_0, exchange_rate_1) = Self::initial_share_exchange_rates(trading_pair);
				let share_to_claim = exchange_rate_0
					.saturating_mul_int(contribution_0)
					.saturating_add(exchange_rate_1.saturating_mul_int(contribution_1));
				T::Currency::transfer(
					trading_pair.dex_share_currency_id(),
					&Self::account_id(),
					who,
					share_to_claim,
				)?;

				// decrease ref count
				frame_system::Pallet::<T>::dec_consumers(who);
			}
			Ok(())
		})
	}

	fn do_add_provision(
		who: &T::AccountId,
		currency_id_a: CurrencyId,
		currency_id_b: CurrencyId,
		contribution_a: Balance,
		contribution_b: Balance,
	) -> DispatchResult {
		let trading_pair =
			TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
		let mut provision_parameters = match Self::trading_pair_statuses(trading_pair) {
			TradingPairStatus::<_, _>::Provisioning(provision_parameters) => provision_parameters,
			_ => return Err(Error::<T>::MustBeProvisioning.into()),
		};
		let (contribution_0, contribution_1) = if currency_id_a == trading_pair.first() {
			(contribution_a, contribution_b)
		} else {
			(contribution_b, contribution_a)
		};

		ensure!(
			contribution_0 >= provision_parameters.min_contribution.0
				|| contribution_1 >= provision_parameters.min_contribution.1,
			Error::<T>::InvalidContributionIncrement
		);

		ProvisioningPool::<T>::try_mutate_exists(trading_pair, &who, |maybe_pool| -> DispatchResult {
			let existed = maybe_pool.is_some();
			let mut pool = maybe_pool.unwrap_or_default();
			pool.0 = pool.0.saturating_add(contribution_0);
			pool.1 = pool.1.saturating_add(contribution_1);

			let module_account_id = Self::account_id();
			T::Currency::transfer(trading_pair.first(), &who, &module_account_id, contribution_0)?;
			T::Currency::transfer(trading_pair.second(), &who, &module_account_id, contribution_1)?;

			*maybe_pool = Some(pool);

			if !existed && maybe_pool.is_some() {
				if frame_system::Pallet::<T>::inc_consumers(&who).is_err() {
					// No providers for the locks. This is impossible under normal circumstances
					// since the funds that are under the lock will themselves be stored in the
					// account and therefore will need a reference.
					log::warn!(
						"Warning: Attempt to introduce lock consumer reference, yet no providers. \
						This is unexpected but should be safe."
					);
				}
			}

			provision_parameters.accumulated_provision.0 = provision_parameters
				.accumulated_provision
				.0
				.saturating_add(contribution_0);
			provision_parameters.accumulated_provision.1 = provision_parameters
				.accumulated_provision
				.1
				.saturating_add(contribution_1);

			TradingPairStatuses::<T>::insert(
				trading_pair,
				TradingPairStatus::<_, _>::Provisioning(provision_parameters),
			);

			Self::deposit_event(Event::AddProvision(
				who.clone(),
				trading_pair.first(),
				contribution_0,
				trading_pair.second(),
				contribution_1,
			));
			Ok(())
		})
	}

	fn do_add_liquidity(
		who: &T::AccountId,
		currency_id_a: CurrencyId,
		currency_id_b: CurrencyId,
		max_amount_a: Balance,
		max_amount_b: Balance,
		min_share_increment: Balance,
		stake_increment_share: bool,
	) -> DispatchResult {
		let trading_pair =
			TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
		ensure!(
			matches!(
				Self::trading_pair_statuses(trading_pair),
				TradingPairStatus::<_, _>::Enabled
			),
			Error::<T>::MustBeEnabled,
		);

		LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> DispatchResult {
			let dex_share_currency_id = trading_pair.dex_share_currency_id();
			let total_shares = T::Currency::total_issuance(dex_share_currency_id);
			let (max_amount_0, max_amount_1) = if currency_id_a == trading_pair.first() {
				(max_amount_a, max_amount_b)
			} else {
				(max_amount_b, max_amount_a)
			};
			let (pool_0_increment, pool_1_increment, share_increment): (Balance, Balance, Balance) = if total_shares
				.is_zero()
			{
				let (exchange_rate_0, exchange_rate_1) = if max_amount_0 > max_amount_1 {
					(
						ExchangeRate::one(),
						ExchangeRate::checked_from_rational(max_amount_0, max_amount_1).unwrap_or_default(),
					)
				} else {
					(
						ExchangeRate::checked_from_rational(max_amount_1, max_amount_0).unwrap_or_default(),
						ExchangeRate::one(),
					)
				};
				let initial_share = exchange_rate_0
					.saturating_mul_int(max_amount_0)
					.saturating_add(exchange_rate_1.saturating_mul_int(max_amount_1));

				(max_amount_0, max_amount_1, initial_share)
			} else {
				let exchange_rate_0_1 = ExchangeRate::checked_from_rational(*pool_1, *pool_0).unwrap_or_default();
				let input_exchange_rate_0_1 =
					ExchangeRate::checked_from_rational(max_amount_1, max_amount_0).unwrap_or_default();

				if input_exchange_rate_0_1 <= exchange_rate_0_1 {
					// max_amount_0 may be too much, calculate the actual amount_0
					let exchange_rate_1_0 = ExchangeRate::checked_from_rational(*pool_0, *pool_1).unwrap_or_default();
					let amount_0 = exchange_rate_1_0.saturating_mul_int(max_amount_1);
					let share_increment = Ratio::checked_from_rational(amount_0, *pool_0)
						.and_then(|n| n.checked_mul_int(total_shares))
						.unwrap_or_default();
					(amount_0, max_amount_1, share_increment)
				} else {
					// max_amount_1 is too much, calculate the actual amount_1
					let amount_1 = exchange_rate_0_1.saturating_mul_int(max_amount_0);
					let share_increment = Ratio::checked_from_rational(amount_1, *pool_1)
						.and_then(|n| n.checked_mul_int(total_shares))
						.unwrap_or_default();
					(max_amount_0, amount_1, share_increment)
				}
			};

			ensure!(
				!share_increment.is_zero() && !pool_0_increment.is_zero() && !pool_1_increment.is_zero(),
				Error::<T>::InvalidLiquidityIncrement,
			);
			ensure!(
				share_increment >= min_share_increment,
				Error::<T>::UnacceptableShareIncrement
			);

			let module_account_id = Self::account_id();
			T::Currency::transfer(trading_pair.first(), who, &module_account_id, pool_0_increment)?;
			T::Currency::transfer(trading_pair.second(), who, &module_account_id, pool_1_increment)?;
			T::Currency::deposit(dex_share_currency_id, who, share_increment)?;

			*pool_0 = pool_0.saturating_add(pool_0_increment);
			*pool_1 = pool_1.saturating_add(pool_1_increment);

			if stake_increment_share {
				T::DEXIncentives::do_deposit_dex_share(who, dex_share_currency_id, share_increment)?;
			}

			Self::deposit_event(Event::AddLiquidity(
				who.clone(),
				trading_pair.first(),
				pool_0_increment,
				trading_pair.second(),
				pool_1_increment,
				share_increment,
			));
			Ok(())
		})
	}

	#[transactional]
	fn do_remove_liquidity(
		who: &T::AccountId,
		currency_id_a: CurrencyId,
		currency_id_b: CurrencyId,
		remove_share: Balance,
		min_withdrawn_a: Balance,
		min_withdrawn_b: Balance,
		by_unstake: bool,
	) -> DispatchResult {
		if remove_share.is_zero() {
			return Ok(());
		}
		let trading_pair =
			TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
		let dex_share_currency_id = trading_pair.dex_share_currency_id();

		LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> DispatchResult {
			let (min_withdrawn_0, min_withdrawn_1) = if currency_id_a == trading_pair.first() {
				(min_withdrawn_a, min_withdrawn_b)
			} else {
				(min_withdrawn_b, min_withdrawn_a)
			};
			let total_shares = T::Currency::total_issuance(dex_share_currency_id);
			let proportion = Ratio::checked_from_rational(remove_share, total_shares).unwrap_or_default();
			let pool_0_decrement = proportion.saturating_mul_int(*pool_0);
			let pool_1_decrement = proportion.saturating_mul_int(*pool_1);
			let module_account_id = Self::account_id();

			ensure!(
				pool_0_decrement >= min_withdrawn_0 && pool_1_decrement >= min_withdrawn_1,
				Error::<T>::UnacceptableLiquidityWithdrawn,
			);

			if by_unstake {
				T::DEXIncentives::do_withdraw_dex_share(who, dex_share_currency_id, remove_share)?;
			}
			T::Currency::withdraw(dex_share_currency_id, &who, remove_share)?;
			T::Currency::transfer(trading_pair.first(), &module_account_id, &who, pool_0_decrement)?;
			T::Currency::transfer(trading_pair.second(), &module_account_id, &who, pool_1_decrement)?;

			*pool_0 = pool_0.saturating_sub(pool_0_decrement);
			*pool_1 = pool_1.saturating_sub(pool_1_decrement);

			Self::deposit_event(Event::RemoveLiquidity(
				who.clone(),
				trading_pair.first(),
				pool_0_decrement,
				trading_pair.second(),
				pool_1_decrement,
				remove_share,
			));
			Ok(())
		})
	}

	fn get_liquidity(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance) {
		if let Some(trading_pair) = TradingPair::from_currency_ids(currency_id_a, currency_id_b) {
			let (pool_0, pool_1) = Self::liquidity_pool(trading_pair);
			if currency_id_a == trading_pair.first() {
				(pool_0, pool_1)
			} else {
				(pool_1, pool_0)
			}
		} else {
			(Zero::zero(), Zero::zero())
		}
	}

	/// Get how much target amount will be got for specific supply amount.
	fn get_target_amount(supply_pool: Balance, target_pool: Balance, supply_amount: Balance) -> Balance {
		if supply_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
			Zero::zero()
		} else {
			let (fee_numerator, fee_denominator) = T::GetExchangeFee::get();
			let supply_amount_with_fee =
				supply_amount.saturating_mul(fee_denominator.saturating_sub(fee_numerator).unique_saturated_into());
			let numerator: U256 = U256::from(supply_amount_with_fee).saturating_mul(U256::from(target_pool));
			let denominator: U256 = U256::from(supply_pool)
				.saturating_mul(U256::from(fee_denominator))
				.saturating_add(U256::from(supply_amount_with_fee));

			numerator
				.checked_div(denominator)
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.unwrap_or_else(Zero::zero)
		}
	}

	/// Get how much supply amount will be paid for specific target amount.
	fn get_supply_amount(supply_pool: Balance, target_pool: Balance, target_amount: Balance) -> Balance {
		if target_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
			Zero::zero()
		} else {
			let (fee_numerator, fee_denominator) = T::GetExchangeFee::get();
			let numerator: U256 = U256::from(supply_pool)
				.saturating_mul(U256::from(target_amount))
				.saturating_mul(U256::from(fee_denominator));
			let denominator: U256 = U256::from(target_pool)
				.saturating_sub(U256::from(target_amount))
				.saturating_mul(U256::from(fee_denominator.saturating_sub(fee_numerator)));

			numerator
				.checked_div(denominator)
				.and_then(|r| r.checked_add(U256::one())) // add 1 to result so that correct the possible losses caused by remainder discarding in
				.and_then(|n| TryInto::<Balance>::try_into(n).ok())
				.unwrap_or_else(Zero::zero)
		}
	}

	fn get_target_amounts(
		path: &[CurrencyId],
		supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Vec<Balance>, DispatchError> {
		let path_length = path.len();
		ensure!(
			path_length >= 2 && path_length <= T::TradingPathLimit::get().saturated_into(),
			Error::<T>::InvalidTradingPathLength
		);
		let mut target_amounts: Vec<Balance> = vec![Zero::zero(); path_length];
		target_amounts[0] = supply_amount;

		let mut i: usize = 0;
		while i + 1 < path_length {
			let trading_pair =
				TradingPair::from_currency_ids(path[i], path[i + 1]).ok_or(Error::<T>::InvalidCurrencyId)?;
			ensure!(
				matches!(
					Self::trading_pair_statuses(trading_pair),
					TradingPairStatus::<_, _>::Enabled
				),
				Error::<T>::MustBeEnabled
			);
			let (supply_pool, target_pool) = Self::get_liquidity(path[i], path[i + 1]);
			ensure!(
				!supply_pool.is_zero() && !target_pool.is_zero(),
				Error::<T>::InsufficientLiquidity
			);
			let target_amount = Self::get_target_amount(supply_pool, target_pool, target_amounts[i]);
			ensure!(!target_amount.is_zero(), Error::<T>::ZeroTargetAmount);

			// invariant check to ensure the constant product formulas (k = x * y)
			let invariant_before_swap: U256 = U256::from(supply_pool).saturating_mul(U256::from(target_pool));
			let invariant_after_swap: U256 = U256::from(supply_pool.saturating_add(target_amounts[i]))
				.saturating_mul(U256::from(target_pool.saturating_sub(target_amount)));
			ensure!(
				invariant_after_swap >= invariant_before_swap,
				Error::<T>::InvariantCheckFailed,
			);

			// check price impact if limit exists
			if let Some(limit) = price_impact_limit {
				let price_impact = Ratio::checked_from_rational(target_amount, target_pool).unwrap_or_else(Ratio::zero);
				ensure!(price_impact <= limit, Error::<T>::ExceedPriceImpactLimit);
			}

			target_amounts[i + 1] = target_amount;
			i += 1;
		}

		Ok(target_amounts)
	}

	fn get_supply_amounts(
		path: &[CurrencyId],
		target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Vec<Balance>, DispatchError> {
		let path_length = path.len();
		ensure!(
			path_length >= 2 && path_length <= T::TradingPathLimit::get().saturated_into(),
			Error::<T>::InvalidTradingPathLength
		);
		let mut supply_amounts: Vec<Balance> = vec![Zero::zero(); path_length];
		supply_amounts[path_length - 1] = target_amount;

		let mut i: usize = path_length - 1;
		while i > 0 {
			let trading_pair =
				TradingPair::from_currency_ids(path[i - 1], path[i]).ok_or(Error::<T>::InvalidCurrencyId)?;
			ensure!(
				matches!(
					Self::trading_pair_statuses(trading_pair),
					TradingPairStatus::<_, _>::Enabled
				),
				Error::<T>::MustBeEnabled
			);
			let (supply_pool, target_pool) = Self::get_liquidity(path[i - 1], path[i]);
			ensure!(
				!supply_pool.is_zero() && !target_pool.is_zero(),
				Error::<T>::InsufficientLiquidity
			);
			let supply_amount = Self::get_supply_amount(supply_pool, target_pool, supply_amounts[i]);
			ensure!(!supply_amount.is_zero(), Error::<T>::ZeroSupplyAmount);

			// invariant check to ensure the constant product formulas (k = x * y)
			let invariant_before_swap: U256 = U256::from(supply_pool).saturating_mul(U256::from(target_pool));
			let invariant_after_swap: U256 = U256::from(supply_pool.saturating_add(supply_amount))
				.saturating_mul(U256::from(target_pool.saturating_sub(supply_amounts[i])));
			ensure!(
				invariant_after_swap >= invariant_before_swap,
				Error::<T>::InvariantCheckFailed,
			);

			// check price impact if limit exists
			if let Some(limit) = price_impact_limit {
				let price_impact =
					Ratio::checked_from_rational(supply_amounts[i], target_pool).unwrap_or_else(Ratio::zero);
				ensure!(price_impact <= limit, Error::<T>::ExceedPriceImpactLimit);
			};

			supply_amounts[i - 1] = supply_amount;
			i -= 1;
		}

		Ok(supply_amounts)
	}

	fn _swap(
		supply_currency_id: CurrencyId,
		target_currency_id: CurrencyId,
		supply_increment: Balance,
		target_decrement: Balance,
	) {
		if let Some(trading_pair) = TradingPair::from_currency_ids(supply_currency_id, target_currency_id) {
			LiquidityPool::<T>::mutate(trading_pair, |(pool_0, pool_1)| {
				if supply_currency_id == trading_pair.first() {
					*pool_0 = pool_0.saturating_add(supply_increment);
					*pool_1 = pool_1.saturating_sub(target_decrement);
				} else {
					*pool_0 = pool_0.saturating_sub(target_decrement);
					*pool_1 = pool_1.saturating_add(supply_increment);
				}
			});
		}
	}

	fn _swap_by_path(path: &[CurrencyId], amounts: &[Balance]) {
		let mut i: usize = 0;
		while i + 1 < path.len() {
			let (supply_currency_id, target_currency_id) = (path[i], path[i + 1]);
			let (supply_increment, target_decrement) = (amounts[i], amounts[i + 1]);
			Self::_swap(
				supply_currency_id,
				target_currency_id,
				supply_increment,
				target_decrement,
			);
			i += 1;
		}
	}

	/// Ensured atomic.
	#[transactional]
	fn do_swap_with_exact_supply(
		who: &T::AccountId,
		path: &[CurrencyId],
		supply_amount: Balance,
		min_target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		let amounts = Self::get_target_amounts(&path, supply_amount, price_impact_limit)?;
		ensure!(
			amounts[amounts.len() - 1] >= min_target_amount,
			Error::<T>::InsufficientTargetAmount
		);
		let module_account_id = Self::account_id();
		let actual_target_amount = amounts[amounts.len() - 1];

		T::Currency::transfer(path[0], who, &module_account_id, supply_amount)?;
		Self::_swap_by_path(&path, &amounts);
		T::Currency::transfer(path[path.len() - 1], &module_account_id, who, actual_target_amount)?;

		Self::deposit_event(Event::Swap(
			who.clone(),
			path.to_vec(),
			supply_amount,
			actual_target_amount,
		));
		Ok(actual_target_amount)
	}

	/// Ensured atomic.
	#[transactional]
	fn do_swap_with_exact_target(
		who: &T::AccountId,
		path: &[CurrencyId],
		target_amount: Balance,
		max_supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		let amounts = Self::get_supply_amounts(&path, target_amount, price_impact_limit)?;
		ensure!(amounts[0] <= max_supply_amount, Error::<T>::ExcessiveSupplyAmount);
		let module_account_id = Self::account_id();
		let actual_supply_amount = amounts[0];

		T::Currency::transfer(path[0], who, &module_account_id, actual_supply_amount)?;
		Self::_swap_by_path(&path, &amounts);
		T::Currency::transfer(path[path.len() - 1], &module_account_id, who, target_amount)?;

		Self::deposit_event(Event::Swap(
			who.clone(),
			path.to_vec(),
			actual_supply_amount,
			target_amount,
		));
		Ok(actual_supply_amount)
	}
}

impl<T: Config> DEXManager<T::AccountId, CurrencyId, Balance> for Pallet<T> {
	fn get_liquidity_pool(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance) {
		Self::get_liquidity(currency_id_a, currency_id_b)
	}

	fn get_liquidity_token_address(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> Option<H160> {
		let trading_pair = TradingPair::from_currency_ids(currency_id_a, currency_id_b)?;
		T::CurrencyIdMapping::encode_evm_address(trading_pair.dex_share_currency_id())
	}

	fn get_swap_target_amount(
		path: &[CurrencyId],
		supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> Option<Balance> {
		Self::get_target_amounts(&path, supply_amount, price_impact_limit)
			.ok()
			.map(|amounts| amounts[amounts.len() - 1])
	}

	fn get_swap_supply_amount(
		path: &[CurrencyId],
		target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> Option<Balance> {
		Self::get_supply_amounts(&path, target_amount, price_impact_limit)
			.ok()
			.map(|amounts| amounts[0])
	}

	fn swap_with_exact_supply(
		who: &T::AccountId,
		path: &[CurrencyId],
		supply_amount: Balance,
		min_target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Self::do_swap_with_exact_supply(who, path, supply_amount, min_target_amount, price_impact_limit)
	}

	fn swap_with_exact_target(
		who: &T::AccountId,
		path: &[CurrencyId],
		target_amount: Balance,
		max_supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Self::do_swap_with_exact_target(who, path, target_amount, max_supply_amount, price_impact_limit)
	}

	// `do_add_liquidity` is used in genesis_build,
	// but transactions are not supported by BasicExternalities,
	// put `transactional` here
	/// Ensured atomic.
	#[transactional]
	fn add_liquidity(
		who: &T::AccountId,
		currency_id_a: CurrencyId,
		currency_id_b: CurrencyId,
		max_amount_a: Balance,
		max_amount_b: Balance,
		min_share_increment: Balance,
		stake_increment_share: bool,
	) -> DispatchResult {
		Self::do_add_liquidity(
			who,
			currency_id_a,
			currency_id_b,
			max_amount_a,
			max_amount_b,
			min_share_increment,
			stake_increment_share,
		)
	}

	fn remove_liquidity(
		who: &T::AccountId,
		currency_id_a: CurrencyId,
		currency_id_b: CurrencyId,
		remove_share: Balance,
		min_withdrawn_a: Balance,
		min_withdrawn_b: Balance,
		by_unstake: bool,
	) -> DispatchResult {
		Self::do_remove_liquidity(
			who,
			currency_id_a,
			currency_id_b,
			remove_share,
			min_withdrawn_a,
			min_withdrawn_b,
			by_unstake,
		)
	}
}
