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

//! # Evm Accounts Module
//!
//! ## Overview
//!
//! Evm Accounts module provide a two way mapping between Substrate accounts and
//! EVM accounts so user only have deal with one account / private key.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use codec::Encode;
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, IsType, OnKilledAccount},
	transactional,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use module_support::AddressMapping;
use orml_traits::currency::TransferAll;
use primitives::{evm::EvmAddress, AccountIndex};
use sp_core::{crypto::AccountId32, ecdsa};
use sp_io::{
	crypto::secp256k1_ecdsa_recover,
	hashing::{blake2_256, keccak_256},
};
use sp_runtime::{
	traits::{LookupError, StaticLookup},
	MultiAddress,
};
use sp_std::{marker::PhantomData, vec::Vec};

mod mock;
mod tests;
pub mod weights;

pub use module::*;
pub use weights::WeightInfo;

pub type EcdsaSignature = ecdsa::Signature;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency for managing Evm account assets.
		type Currency: Currency<Self::AccountId>;

		/// Mapping from address to account id.
		type AddressMapping: AddressMapping<Self::AccountId>;

		/// Merge free balance from source to dest.
		type TransferAll: TransferAll<Self::AccountId>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Mapping between Substrate accounts and EVM accounts
		/// claim account.
		ClaimAccount {
			account_id: T::AccountId,
			evm_address: EvmAddress,
		},
	}

	/// Error for evm accounts module.
	#[pallet::error]
	pub enum Error<T> {
		/// AccountId has mapped
		AccountIdHasMapped,
		/// Eth address has mapped
		EthAddressHasMapped,
		/// Bad signature
		BadSignature,
		/// Invalid signature
		InvalidSignature,
		/// Account ref count is not zero
		NonZeroRefCount,
	}

	/// The Substrate Account for EvmAddresses
	///
	/// Accounts: map EvmAddress => Option<AccountId>
	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub type Accounts<T: Config> = StorageMap<_, Twox64Concat, EvmAddress, T::AccountId, OptionQuery>;

	/// The EvmAddress for Substrate Accounts
	///
	/// EvmAddresses: map AccountId => Option<EvmAddress>
	#[pallet::storage]
	#[pallet::getter(fn evm_addresses)]
	pub type EvmAddresses<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, EvmAddress, OptionQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Claim account mapping between Substrate accounts and EVM accounts.
		/// Ensure eth_address has not been mapped.
		///
		/// - `eth_address`: The address to bind to the caller's account
		/// - `eth_signature`: A signature generated by the address to prove ownership
		#[pallet::weight(T::WeightInfo::claim_account())]
		#[transactional]
		pub fn claim_account(
			origin: OriginFor<T>,
			eth_address: EvmAddress,
			eth_signature: EcdsaSignature,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// ensure account_id and eth_address has not been mapped
			ensure!(!EvmAddresses::<T>::contains_key(&who), Error::<T>::AccountIdHasMapped);
			ensure!(
				!Accounts::<T>::contains_key(eth_address),
				Error::<T>::EthAddressHasMapped
			);

			// recover evm address from signature
			let address = Self::eth_recover(&eth_signature, &who.using_encoded(to_ascii_hex), &[][..])
				.ok_or(Error::<T>::BadSignature)?;
			ensure!(eth_address == address, Error::<T>::InvalidSignature);

			// check if the evm padded address already exists
			let account_id = T::AddressMapping::get_account_id(&eth_address);
			if frame_system::Pallet::<T>::account_exists(&account_id) {
				// merge balance from `evm padded address` to `origin`
				T::TransferAll::transfer_all(&account_id, &who)?;
			}

			Accounts::<T>::insert(eth_address, &who);
			EvmAddresses::<T>::insert(&who, eth_address);

			Self::deposit_event(Event::ClaimAccount {
				account_id: who,
				evm_address: eth_address,
			});

			Ok(())
		}

		/// Claim account mapping between Substrate accounts and a generated EVM
		/// address based off of those accounts.
		/// Ensure eth_address has not been mapped
		#[pallet::weight(T::WeightInfo::claim_default_account())]
		pub fn claim_default_account(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// ensure account_id has not been mapped
			ensure!(!EvmAddresses::<T>::contains_key(&who), Error::<T>::AccountIdHasMapped);

			let eth_address = T::AddressMapping::get_or_create_evm_address(&who);

			Self::deposit_event(Event::ClaimAccount {
				account_id: who,
				evm_address: eth_address,
			});

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign`
	// would sign.
	pub fn ethereum_signable_message(what: &[u8], extra: &[u8]) -> Vec<u8> {
		let prefix = b"acala evm:";
		let mut l = prefix.len() + what.len() + extra.len();
		let mut rev = Vec::new();
		while l > 0 {
			rev.push(b'0' + (l % 10) as u8);
			l /= 10;
		}
		let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
		v.extend(rev.into_iter().rev());
		v.extend_from_slice(&prefix[..]);
		v.extend_from_slice(what);
		v.extend_from_slice(extra);
		v
	}

	// Attempts to recover the Ethereum address from a message signature signed by
	// using the Ethereum RPC's `personal_sign` and `eth_sign`.
	pub fn eth_recover(s: &EcdsaSignature, what: &[u8], extra: &[u8]) -> Option<EvmAddress> {
		let msg = keccak_256(&Self::ethereum_signable_message(what, extra));
		let mut res = EvmAddress::default();
		res.0
			.copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(&s.0, &msg).ok()?[..])[12..]);
		Some(res)
	}

	#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
	// Returns an Etherum public key derived from an Ethereum secret key.
	pub fn eth_public(secret: &libsecp256k1::SecretKey) -> libsecp256k1::PublicKey {
		libsecp256k1::PublicKey::from_secret_key(secret)
	}

	#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
	// Returns an Etherum address derived from an Ethereum secret key.
	// Only for tests
	pub fn eth_address(secret: &libsecp256k1::SecretKey) -> EvmAddress {
		EvmAddress::from_slice(&keccak_256(&Self::eth_public(secret).serialize()[1..65])[12..])
	}

	#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
	// Constructs a message and signs it.
	pub fn eth_sign(secret: &libsecp256k1::SecretKey, what: &[u8], extra: &[u8]) -> EcdsaSignature {
		let msg = keccak_256(&Self::ethereum_signable_message(&to_ascii_hex(what)[..], extra));
		let (sig, recovery_id) = libsecp256k1::sign(&libsecp256k1::Message::parse(&msg), secret);
		let mut r = [0u8; 65];
		r[0..64].copy_from_slice(&sig.serialize()[..]);
		r[64] = recovery_id.serialize();
		EcdsaSignature::from_slice(&r)
	}
}

// Creates a an EvmAddress from an AccountId by appending the bytes "evm:" to
// the account_id and hashing it.
fn account_to_default_evm_address(account_id: &impl Encode) -> EvmAddress {
	let payload = (b"evm:", account_id);
	EvmAddress::from_slice(&payload.using_encoded(blake2_256)[0..20])
}

pub struct EvmAddressMapping<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> AddressMapping<T::AccountId> for EvmAddressMapping<T>
where
	T::AccountId: IsType<AccountId32>,
{
	// Returns the AccountId used go generate the given EvmAddress.
	fn get_account_id(address: &EvmAddress) -> T::AccountId {
		if let Some(acc) = Accounts::<T>::get(address) {
			acc
		} else {
			let mut data: [u8; 32] = [0u8; 32];
			data[0..4].copy_from_slice(b"evm:");
			data[4..24].copy_from_slice(&address[..]);
			AccountId32::from(data).into()
		}
	}

	// Returns the EvmAddress associated with a given AccountId or the
	// underlying EvmAddress of the AccountId.
	// Returns None if there is no EvmAddress associated with the AccountId
	// and there is no underlying EvmAddress in the AccountId.
	fn get_evm_address(account_id: &T::AccountId) -> Option<EvmAddress> {
		// Return the EvmAddress if a mapping to account_id exists
		EvmAddresses::<T>::get(account_id).or_else(|| {
			let data: &[u8] = account_id.into_ref().as_ref();
			// Return the underlying EVM address if it exists otherwise return None
			if data.starts_with(b"evm:") {
				Some(EvmAddress::from_slice(&data[4..24]))
			} else {
				None
			}
		})
	}

	// Returns the EVM address associated with an account ID and generates an
	// account mapping if no association exists.
	fn get_or_create_evm_address(account_id: &T::AccountId) -> EvmAddress {
		Self::get_evm_address(account_id).unwrap_or_else(|| {
			let addr = account_to_default_evm_address(account_id);

			// create reverse mapping
			Accounts::<T>::insert(&addr, &account_id);
			EvmAddresses::<T>::insert(&account_id, &addr);

			addr
		})
	}

	// Returns the default EVM address associated with an account ID.
	fn get_default_evm_address(account_id: &T::AccountId) -> EvmAddress {
		account_to_default_evm_address(account_id)
	}

	// Returns true if a given AccountId is associated with a given EvmAddress
	// and false if is not.
	fn is_linked(account_id: &T::AccountId, evm: &EvmAddress) -> bool {
		Self::get_evm_address(account_id).as_ref() == Some(evm)
			|| &account_to_default_evm_address(account_id.into_ref()) == evm
	}
}

pub struct CallKillAccount<T>(PhantomData<T>);
impl<T: Config> OnKilledAccount<T::AccountId> for CallKillAccount<T> {
	fn on_killed_account(who: &T::AccountId) {
		// remove the reserve mapping that could be created by
		// `get_or_create_evm_address`
		Accounts::<T>::remove(account_to_default_evm_address(who.into_ref()));

		// remove mapping created by `claim_account`
		if let Some(evm_addr) = Pallet::<T>::evm_addresses(who) {
			Accounts::<T>::remove(evm_addr);
			EvmAddresses::<T>::remove(who);
		}
	}
}

impl<T: Config> StaticLookup for Pallet<T> {
	type Source = MultiAddress<T::AccountId, AccountIndex>;
	type Target = T::AccountId;

	fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
		match a {
			MultiAddress::Address20(i) => Ok(T::AddressMapping::get_account_id(&EvmAddress::from_slice(&i))),
			_ => Err(LookupError),
		}
	}

	fn unlookup(a: Self::Target) -> Self::Source {
		MultiAddress::Id(a)
	}
}

/// Converts the given binary data into ASCII-encoded hex. It will be twice
/// the length.
pub fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
	let mut r = Vec::with_capacity(data.len() * 2);
	let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
	for &b in data.iter() {
		push_nibble(b / 16);
		push_nibble(b % 16);
	}
	r
}
