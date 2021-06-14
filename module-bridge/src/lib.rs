//! Bridge runtime module.
#![deny(rust_2018_idioms)]

use std::collections::{BTreeMap, BTreeSet};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use oasis_runtime_sdk::{
    self as sdk,
    context::{Context, TxContext},
    core::common::cbor,
    crypto::signature::PublicKey,
    error::{self, Error as _},
    module::{self, Module as _},
    modules, storage,
    types::{address::Address, token, transaction::CallResult},
};

#[cfg(test)]
mod test;
pub mod types;

/// Unique module name.
const MODULE_NAME: &str = "bridge";

/// Errors emitted by the accounts module.
#[derive(Error, Debug, sdk::Error)]
pub enum Error {
    #[error("invalid argument")]
    #[sdk_error(code = 1)]
    InvalidArgument,

    #[error("not authorized")]
    #[sdk_error(code = 2)]
    NotAuthorized,

    #[error("invalid sequence number")]
    #[sdk_error(code = 3)]
    InvalidSequenceNumber,

    #[error("insufficient balance")]
    #[sdk_error(code = 4)]
    InsufficientBalance,

    #[error("witness already submitted signature")]
    #[sdk_error(code = 5)]
    AlreadySubmittedSignature,

    #[error("unsupported denomination")]
    #[sdk_error(code = 6)]
    UnsupportedDenomination,
}

impl From<modules::accounts::Error> for Error {
    fn from(error: modules::accounts::Error) -> Self {
        match error {
            modules::accounts::Error::InsufficientBalance => Error::InsufficientBalance,
            _ => Error::InvalidArgument,
        }
    }
}

/// Events emitted by the accounts module.
#[derive(Debug, Serialize, Deserialize, sdk::Event)]
#[serde(untagged)]
pub enum Event {
    #[sdk_event(code = 1)]
    Lock {
        id: u64,
        owner: Address,
        target: types::RemoteAddress,
        amount: token::BaseUnits,
    },

    #[sdk_event(code = 2)]
    Release {
        id: u64,
        target: Address,
        amount: token::BaseUnits,
    },

    #[sdk_event(code = 3)]
    WitnessesSigned(types::WitnessSignatures),
}

/// Parameters for the bridge module.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// A list of authorized witness public keys.
    #[serde(rename = "witnesses")]
    pub witnesses: Vec<PublicKey>,

    /// Number of witnesses that needs to sign off.
    #[serde(rename = "threshold")]
    pub threshold: u64,

    /// Denominations local to this side of the bridge.
    #[serde(rename = "local_denominations")]
    pub local_denominations: BTreeSet<token::Denomination>,

    /// Denominations that exist on the remote side of the bridge.
    #[serde(rename = "remote_denominations")]
    pub remote_denominations: BTreeMap<token::Denomination, types::RemoteDenomination>,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            witnesses: vec![],
            threshold: 1,
            local_denominations: BTreeSet::new(),
            remote_denominations: BTreeMap::new(),
        }
    }
}

/// Errors emitted by the accounts module.
#[derive(Error, Debug)]
pub enum ParameterValidationError {
    #[error("too many witnesses")]
    TooManyWitnesses,
    #[error("a denomination cannot be both local and remote")]
    DenominationLocalAndRemote,
}

impl module::Parameters for Parameters {
    type Error = ParameterValidationError;

    fn validate_basic(&self) -> Result<(), Self::Error> {
        if self.witnesses.len() > (u16::MAX as usize) {
            return Err(ParameterValidationError::TooManyWitnesses);
        }

        // Make sure a denomination is either local or remote, but not both.
        for rd in self.remote_denominations.keys() {
            if self.local_denominations.contains(rd) {
                return Err(ParameterValidationError::DenominationLocalAndRemote);
            }
        }

        Ok(())
    }
}

/// Genesis state for the bridge module.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Genesis {
    #[serde(rename = "parameters")]
    pub parameters: Parameters,
}

impl Default for Genesis {
    fn default() -> Self {
        Self {
            parameters: Default::default(),
        }
    }
}

/// State schema constants.
pub mod state {
    /// Next outgoing (to external chain) sequence number.
    pub const NEXT_OUT_SEQUENCE: &[u8] = &[0x01];
    /// Next incoming (from external chain) sequence number.
    pub const NEXT_IN_SEQUENCE: &[u8] = &[0x02];

    /// Map of outgoing sequence number to list of witness signatures.
    pub const OUT_WITNESS_SIGNATURES: &[u8] = &[0x03];
    /// Map of incoming sequence number to list of witness signatures.
    pub const IN_WITNESS_SIGNATURES: &[u8] = &[0x04];
}

pub struct Module<Accounts: modules::accounts::API> {
    _accounts: std::marker::PhantomData<Accounts>,
}

lazy_static! {
    /// Module's address where all locked funds are stored.
    pub static ref ADDRESS_LOCKED_FUNDS: Address = Address::from_module(MODULE_NAME, "locked-funds");
}

impl<Accounts: modules::accounts::API> Module<Accounts> {
    fn ensure_local_or_remote<C: Context>(
        ctx: &mut C,
        denomination: &token::Denomination,
    ) -> Result<Option<types::RemoteDenomination>, Error> {
        let params = Self::params(ctx.runtime_state());

        // Check if the given denomination is local.
        if params.local_denominations.contains(denomination) {
            return Ok(None);
        }
        // Check if the given denomination is remote.
        if let Some(remote) = params.remote_denominations.get(denomination) {
            return Ok(Some(remote.clone()));
        }

        Err(Error::UnsupportedDenomination)
    }

    fn tx_lock<C: TxContext>(ctx: &mut C, body: types::Lock) -> Result<types::LockResult, Error> {
        let remote = Self::ensure_local_or_remote(ctx, body.amount.denomination())?;
        let caller_address = ctx.tx_caller_address();

        if ctx.is_check_only() {
            return Ok(types::LockResult { id: 0 });
        }

        // Transfer funds from user's account into the bridge-owned account.
        Accounts::transfer(ctx, caller_address, *ADDRESS_LOCKED_FUNDS, &body.amount)?;

        // Assign a unique identifier to the event.
        let mut store = storage::PrefixStore::new(ctx.runtime_state(), &MODULE_NAME);
        let mut tstore = storage::TypedStore::new(&mut store);
        let id: u64 = tstore.get(state::NEXT_OUT_SEQUENCE).unwrap_or_default();
        tstore.insert(state::NEXT_OUT_SEQUENCE, &(id + 1));

        // Create an entry in outgoing witness signatures map.
        let amount = body.amount.clone();
        let target = body.target;
        let mut out_witness_signatures = storage::TypedStore::new(storage::PrefixStore::new(
            &mut store,
            &state::OUT_WITNESS_SIGNATURES,
        ));
        out_witness_signatures.insert(
            id.to_storage_key(),
            &types::WitnessSignatures::new(id, types::Operation::Lock(body)),
        );

        // If this is a remote denomination burn the amount from the bridge-owned account. If this
        // is a local denomination, then the amount just stays locked in the account.
        if remote.is_some() {
            Accounts::burn(ctx, *ADDRESS_LOCKED_FUNDS, &amount)?;
        }

        // Emit a lock event.
        ctx.emit_event(Event::Lock {
            id,
            owner: caller_address,
            target,
            amount,
        });

        Ok(types::LockResult { id })
    }

    fn tx_witness<C: TxContext>(ctx: &mut C, body: types::Witness) -> Result<(), Error> {
        if ctx.is_check_only() {
            return Ok(());
        }

        let caller_address = ctx.tx_caller_address();
        let params = Self::params(ctx.runtime_state());
        // Make sure the caller is an authorized witness.
        let (index, _pk) = params
            .witnesses
            .iter()
            .enumerate()
            .find(|(_, pk)| Address::from_pk(pk) == caller_address)
            .ok_or(Error::NotAuthorized)?;

        // Check if sequence number is correct.
        let mut store = storage::PrefixStore::new(ctx.runtime_state(), &MODULE_NAME);
        let mut out_witness_signatures = storage::TypedStore::new(storage::PrefixStore::new(
            &mut store,
            &state::OUT_WITNESS_SIGNATURES,
        ));
        let mut info: types::WitnessSignatures = out_witness_signatures
            .get(body.id.to_storage_key())
            .ok_or(Error::InvalidSequenceNumber)?;

        // Make sure it didn't already submit a signature.
        if info.witnesses.iter().any(|i| *i as usize == index) {
            return Err(Error::AlreadySubmittedSignature);
        }
        // TODO: Validate witness signature.
        // TODO: Verify signature against the remote denomination.

        // Store signature in storage.
        info.witnesses.push(index as u16);
        info.signatures.push(body.signature);
        // Check if there's enough signatures.
        if (info.witnesses.len() as u64) < params.threshold {
            // Not enough signatures yet.
            out_witness_signatures.insert(body.id.to_storage_key(), &info);
            return Ok(());
        }

        // Clear entry in storage.
        out_witness_signatures.remove(body.id.to_storage_key());

        // Emit the collected signatures.
        ctx.emit_event(Event::WitnessesSigned(info));

        Ok(())
    }

    fn tx_release<C: TxContext>(ctx: &mut C, body: types::Release) -> Result<(), Error> {
        let remote = Self::ensure_local_or_remote(ctx, body.amount.denomination())?;
        let caller_address = ctx.tx_caller_address();

        if ctx.is_check_only() {
            return Ok(());
        }

        let params = Self::params(ctx.runtime_state());
        // Make sure the caller is an authorized witness.
        let (index, _pk) = params
            .witnesses
            .iter()
            .enumerate()
            .find(|(_, pk)| Address::from_pk(pk) == caller_address)
            .ok_or(Error::NotAuthorized)?;
        let index = index as u16;

        // Check if sequence number is correct. This requires that all events are processed in
        // sequence by the witnesses and no events are missed.
        let mut store = storage::PrefixStore::new(ctx.runtime_state(), &MODULE_NAME);
        let tstore = storage::TypedStore::new(&mut store);
        let expected_id: u64 = tstore.get(state::NEXT_IN_SEQUENCE).unwrap_or_default();
        if body.id != expected_id {
            return Err(Error::InvalidSequenceNumber);
        }

        // Fetch existing signatures.
        let mut in_witness_signatures = storage::TypedStore::new(storage::PrefixStore::new(
            &mut store,
            &state::IN_WITNESS_SIGNATURES,
        ));
        let mut info: types::IncomingWitnessSignatures = in_witness_signatures
            .get(body.id.to_storage_key())
            .unwrap_or_default();

        // Make sure it didn't already submit a signature.
        if info.witnesses.iter().any(|i| i == &index) {
            return Err(Error::AlreadySubmittedSignature);
        }

        // There can be multiple different operations proposed for the sequence (in case some
        // witnesses are corrupted). We handle these by hashing the operation and using that as the
        // discriminator.
        let op = types::Operation::Release(body.clone());
        let op_id = types::OperationId::from(&op);
        let op_sigs = info
            .ops
            .entry(op_id)
            .or_insert_with(|| types::WitnessSignatures::new(body.id, op));
        // TODO: Validate witness signature.
        // TODO: Verify signature against the remote denomination.

        // Store which witnesses signed in storage. Note that in the incoming case we don't need to
        // store the actual signatures as we verify them here and no longer need them.
        info.witnesses.push(index);
        op_sigs.witnesses.push(index);
        // Check if there's enough signatures.
        if (op_sigs.witnesses.len() as u64) < params.threshold {
            // Not enough signatures yet.
            in_witness_signatures.insert(body.id.to_storage_key(), &info);
            return Ok(());
        }

        // Clear entry in storage.
        in_witness_signatures.remove(body.id.to_storage_key());

        // Increment sequence number.
        let mut tstore = storage::TypedStore::new(&mut store);
        tstore.insert(state::NEXT_IN_SEQUENCE, &(expected_id + 1));

        // If this is a remote denomination mint the amount in the bridge-owned account. If this is
        // a local denomination, then the amount is just unlocked from the account.
        if let Some(_remote) = remote {
            Accounts::mint(ctx, *ADDRESS_LOCKED_FUNDS, &body.amount)?;
        }

        // Transfer funds from bridge-owned account into user's account.
        Accounts::transfer(ctx, *ADDRESS_LOCKED_FUNDS, body.target, &body.amount)?;

        // Emit release event.
        ctx.emit_event(Event::Release {
            id: body.id,
            target: body.target,
            amount: body.amount,
        });

        Ok(())
    }

    fn query_next_sequence_numbers<C: Context>(
        ctx: &mut C,
        _args: (),
    ) -> Result<types::NextSequenceNumbers, Error> {
        let store = storage::PrefixStore::new(ctx.runtime_state(), &MODULE_NAME);
        let store = storage::TypedStore::new(store);

        Ok(types::NextSequenceNumbers {
            incoming: store.get(state::NEXT_IN_SEQUENCE).unwrap_or_default(),
            outgoing: store.get(state::NEXT_OUT_SEQUENCE).unwrap_or_default(),
        })
    }

    fn query_parameters<C: Context>(ctx: &mut C, _args: ()) -> Result<Parameters, Error> {
        Ok(Self::params(ctx.runtime_state()))
    }
}

impl<Accounts: modules::accounts::API> module::Module for Module<Accounts> {
    const NAME: &'static str = MODULE_NAME;
    type Error = Error;
    type Event = Event;
    type Parameters = Parameters;
}

impl<Accounts: modules::accounts::API> module::MethodHandler for Module<Accounts> {
    fn dispatch_call<C: TxContext>(
        ctx: &mut C,
        method: &str,
        body: cbor::Value,
    ) -> module::DispatchResult<cbor::Value, CallResult> {
        match method {
            "bridge.Lock" => {
                let result = || -> Result<cbor::Value, Error> {
                    let args = cbor::from_value(body).map_err(|_| Error::InvalidArgument)?;
                    Ok(cbor::to_value(&Self::tx_lock(ctx, args)?))
                }();
                match result {
                    Ok(value) => module::DispatchResult::Handled(CallResult::Ok(value)),
                    Err(err) => module::DispatchResult::Handled(err.to_call_result()),
                }
            }
            "bridge.Witness" => {
                let result = || -> Result<cbor::Value, Error> {
                    let args = cbor::from_value(body).map_err(|_| Error::InvalidArgument)?;
                    Ok(cbor::to_value(&Self::tx_witness(ctx, args)?))
                }();
                match result {
                    Ok(value) => module::DispatchResult::Handled(CallResult::Ok(value)),
                    Err(err) => module::DispatchResult::Handled(err.to_call_result()),
                }
            }
            "bridge.Release" => {
                let result = || -> Result<cbor::Value, Error> {
                    let args = cbor::from_value(body).map_err(|_| Error::InvalidArgument)?;
                    Ok(cbor::to_value(&Self::tx_release(ctx, args)?))
                }();
                match result {
                    Ok(value) => module::DispatchResult::Handled(CallResult::Ok(value)),
                    Err(err) => module::DispatchResult::Handled(err.to_call_result()),
                }
            }
            _ => module::DispatchResult::Unhandled(body),
        }
    }

    fn dispatch_query<C: Context>(
        ctx: &mut C,
        method: &str,
        args: cbor::Value,
    ) -> module::DispatchResult<cbor::Value, Result<cbor::Value, error::RuntimeError>> {
        match method {
            "bridge.NextSequenceNumbers" => module::DispatchResult::Handled((|| {
                let args = cbor::from_value(args).map_err(|_| Error::InvalidArgument)?;
                Ok(cbor::to_value(&Self::query_next_sequence_numbers(
                    ctx, args,
                )?))
            })()),
            "bridge.Parameters" => module::DispatchResult::Handled((|| {
                let args = cbor::from_value(args).map_err(|_| Error::InvalidArgument)?;
                Ok(cbor::to_value(&Self::query_parameters(ctx, args)?))
            })()),
            _ => module::DispatchResult::Unhandled(args),
        }
    }
}

impl<Accounts: modules::accounts::API> Module<Accounts> {
    fn init<C: Context>(ctx: &mut C, genesis: &Genesis) {
        // Set genesis parameters.
        Self::set_params(ctx.runtime_state(), &genesis.parameters);
    }

    fn migrate<C: Context>(_ctx: &mut C, _from: u32) -> bool {
        // No migrations currently supported.
        false
    }
}

impl<Accounts: modules::accounts::API> module::MigrationHandler for Module<Accounts> {
    type Genesis = Genesis;

    fn init_or_migrate<C: Context>(
        ctx: &mut C,
        meta: &mut modules::core::types::Metadata,
        genesis: &Self::Genesis,
    ) -> bool {
        let version = meta.versions.get(Self::NAME).copied().unwrap_or_default();
        if version == 0 {
            // Initialize state from genesis.
            Self::init(ctx, genesis);
            meta.versions.insert(Self::NAME.to_owned(), Self::VERSION);
            return true;
        }

        Self::migrate(ctx, version)
    }
}

impl<Accounts: modules::accounts::API> module::AuthHandler for Module<Accounts> {}

impl<Accounts: modules::accounts::API> module::BlockHandler for Module<Accounts> {}

/// A trait that exist solely to convert u64 IDs to bytes for use as a storage key.
/// Method call syntax is easier to read than alternatives like macro/function invocations
/// and wrapper types.
trait ToStorageKey {
    fn to_storage_key(&self) -> [u8; 8];
}

impl ToStorageKey for u64 {
    fn to_storage_key(&self) -> [u8; 8] {
        self.to_be_bytes()
    }
}
