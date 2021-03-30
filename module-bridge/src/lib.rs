//! Bridge runtime module.
use std::collections::{BTreeMap, BTreeSet};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use oasis_runtime_sdk::{
    context::{DispatchContext, TxContext},
    core::common::cbor,
    crypto::signature::PublicKey,
    error::{self, Error as _},
    event,
    module::{self, CallableMethodInfo, Module as _, QueryMethodInfo},
    modules, storage,
    types::{address::Address, token, transaction::CallResult},
};

#[cfg(test)]
mod test;
pub mod types;

/// Unique module name.
const MODULE_NAME: &str = "bridge";

// TODO: Add a custom derive macro for easier error derivation (module/error codes).
/// Errors emitted by the accounts module.
#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid argument")]
    InvalidArgument,
    #[error("not authorized")]
    NotAuthorized,
    #[error("invalid sequence number")]
    InvalidSequenceNumber,
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("witness already submitted signature")]
    AlreadySubmittedSignature,
    #[error("unsupported denomination")]
    UnsupportedDenomination,
}

impl error::Error for Error {
    fn module(&self) -> &str {
        MODULE_NAME
    }

    fn code(&self) -> u32 {
        match self {
            Error::InvalidArgument => 1,
            Error::NotAuthorized => 2,
            Error::InvalidSequenceNumber => 3,
            Error::InsufficientBalance => 4,
            Error::AlreadySubmittedSignature => 5,
            Error::UnsupportedDenomination => 6,
        }
    }
}

impl From<modules::accounts::Error> for Error {
    fn from(error: modules::accounts::Error) -> Self {
        match error {
            modules::accounts::Error::InsufficientBalance => Error::InsufficientBalance,
            _ => Error::InvalidArgument,
        }
    }
}

impl From<Error> for error::RuntimeError {
    fn from(err: Error) -> error::RuntimeError {
        error::RuntimeError::new(err.module(), err.code(), &err.msg())
    }
}

// TODO: Add a custom derive macro for easier event derivation (tags).
/// Events emitted by the accounts module.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Event {
    Lock {
        id: u64,
        owner: Address,
        target: types::RemoteAddress,
        amount: token::BaseUnits,
    },

    Release {
        id: u64,
        target: Address,
        amount: token::BaseUnits,
    },

    WitnessesSigned(types::WitnessSignatures),
}

impl event::Event for Event {
    fn module(&self) -> &str {
        MODULE_NAME
    }

    fn code(&self) -> u32 {
        match self {
            Event::Lock { .. } => 1,
            Event::Release { .. } => 2,
            Event::WitnessesSigned(_) => 3,
        }
    }

    fn value(&self) -> cbor::Value {
        cbor::to_value(self)
    }
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
    fn ensure_local_or_remote(
        ctx: &mut TxContext,
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

    fn tx_lock(ctx: &mut TxContext, body: types::Lock) -> Result<types::LockResult, Error> {
        let remote = Self::ensure_local_or_remote(ctx, body.amount.denomination())?;

        if ctx.is_check_only() {
            return Ok(types::LockResult { id: 0 });
        }

        // Transfer funds from user's account into the bridge-owned account.
        Accounts::transfer(
            ctx,
            ctx.tx_caller_address(),
            *ADDRESS_LOCKED_FUNDS,
            &body.amount,
        )?;

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
            &storage::OwnedStoreKey::from(id),
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
            owner: ctx.tx_caller_address(),
            target,
            amount,
        });

        Ok(types::LockResult { id })
    }

    fn tx_witness(ctx: &mut TxContext, body: types::Witness) -> Result<(), Error> {
        if ctx.is_check_only() {
            return Ok(());
        }

        let params = Self::params(ctx.runtime_state());
        // Make sure the caller is an authorized witness.
        let (index, _pk) = params
            .witnesses
            .iter()
            .enumerate()
            .find(|(_, pk)| Address::from_pk(pk) == ctx.tx_caller_address())
            .ok_or(Error::NotAuthorized)?;

        // Check if sequence number is correct.
        let mut store = storage::PrefixStore::new(ctx.runtime_state(), &MODULE_NAME);
        let mut out_witness_signatures = storage::TypedStore::new(storage::PrefixStore::new(
            &mut store,
            &state::OUT_WITNESS_SIGNATURES,
        ));
        let mut info: types::WitnessSignatures = out_witness_signatures
            .get(&storage::OwnedStoreKey::from(body.id))
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
            out_witness_signatures.insert(&storage::OwnedStoreKey::from(body.id), &info);
            return Ok(());
        }

        // Clear entry in storage.
        out_witness_signatures.remove(&storage::OwnedStoreKey::from(body.id));

        // Emit the collected signatures.
        ctx.emit_event(Event::WitnessesSigned(info));

        Ok(())
    }

    fn tx_release(ctx: &mut TxContext, body: types::Release) -> Result<(), Error> {
        let remote = Self::ensure_local_or_remote(ctx, body.amount.denomination())?;

        if ctx.is_check_only() {
            return Ok(());
        }

        let params = Self::params(ctx.runtime_state());
        // Make sure the caller is an authorized witness.
        let (index, _pk) = params
            .witnesses
            .iter()
            .enumerate()
            .find(|(_, pk)| Address::from_pk(pk) == ctx.tx_caller_address())
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
            .get(&storage::OwnedStoreKey::from(body.id))
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
            in_witness_signatures.insert(&storage::OwnedStoreKey::from(body.id), &info);
            return Ok(());
        }

        // Clear entry in storage.
        in_witness_signatures.remove(&storage::OwnedStoreKey::from(body.id));

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

    fn query_next_sequence_numbers(
        ctx: &mut DispatchContext,
        _args: (),
    ) -> Result<types::NextSequenceNumbers, Error> {
        let store = storage::PrefixStore::new(ctx.runtime_state(), &MODULE_NAME);
        let store = storage::TypedStore::new(store);

        Ok(types::NextSequenceNumbers {
            incoming: store.get(state::NEXT_IN_SEQUENCE).unwrap_or_default(),
            outgoing: store.get(state::NEXT_OUT_SEQUENCE).unwrap_or_default(),
        })
    }

    fn query_parameters(ctx: &mut DispatchContext, _args: ()) -> Result<Parameters, Error> {
        Ok(Self::params(ctx.runtime_state()))
    }
}

impl<Accounts: modules::accounts::API> Module<Accounts> {
    fn _callable_lock_handler(
        _mi: &CallableMethodInfo,
        ctx: &mut TxContext,
        body: cbor::Value,
    ) -> CallResult {
        let result = || -> Result<cbor::Value, Error> {
            let args = cbor::from_value(body).map_err(|_| Error::InvalidArgument)?;
            Ok(cbor::to_value(&Self::tx_lock(ctx, args)?))
        }();
        match result {
            Ok(value) => CallResult::Ok(value),
            Err(err) => err.to_call_result(),
        }
    }

    fn _callable_witness_handler(
        _mi: &CallableMethodInfo,
        ctx: &mut TxContext,
        body: cbor::Value,
    ) -> CallResult {
        let result = || -> Result<cbor::Value, Error> {
            let args = cbor::from_value(body).map_err(|_| Error::InvalidArgument)?;
            Ok(cbor::to_value(&Self::tx_witness(ctx, args)?))
        }();
        match result {
            Ok(value) => CallResult::Ok(value),
            Err(err) => err.to_call_result(),
        }
    }

    fn _callable_release_handler(
        _mi: &CallableMethodInfo,
        ctx: &mut TxContext,
        body: cbor::Value,
    ) -> CallResult {
        let result = || -> Result<cbor::Value, Error> {
            let args = cbor::from_value(body).map_err(|_| Error::InvalidArgument)?;
            Ok(cbor::to_value(&Self::tx_release(ctx, args)?))
        }();
        match result {
            Ok(value) => CallResult::Ok(value),
            Err(err) => err.to_call_result(),
        }
    }

    fn _query_next_sequence_numbers_handler(
        _mi: &QueryMethodInfo,
        ctx: &mut DispatchContext,
        args: cbor::Value,
    ) -> Result<cbor::Value, error::RuntimeError> {
        let args = cbor::from_value(args).map_err(|_| Error::InvalidArgument)?;
        Ok(cbor::to_value(&Self::query_next_sequence_numbers(
            ctx, args,
        )?))
    }

    fn _query_parameters_handler(
        _mi: &QueryMethodInfo,
        ctx: &mut DispatchContext,
        args: cbor::Value,
    ) -> Result<cbor::Value, error::RuntimeError> {
        let args = cbor::from_value(args).map_err(|_| Error::InvalidArgument)?;
        Ok(cbor::to_value(&Self::query_parameters(ctx, args)?))
    }
}

impl<Accounts: modules::accounts::API> module::Module for Module<Accounts> {
    const NAME: &'static str = MODULE_NAME;
    type Error = Error;
    type Event = Event;
    type Parameters = Parameters;
}

impl<Accounts: modules::accounts::API> module::MethodRegistrationHandler for Module<Accounts> {
    fn register_methods(methods: &mut module::MethodRegistry) {
        // Callable methods.
        methods.register_callable(module::CallableMethodInfo {
            name: "bridge.Lock",
            handler: Self::_callable_lock_handler,
        });
        methods.register_callable(module::CallableMethodInfo {
            name: "bridge.Witness",
            handler: Self::_callable_witness_handler,
        });
        methods.register_callable(module::CallableMethodInfo {
            name: "bridge.Release",
            handler: Self::_callable_release_handler,
        });

        // Queries.
        methods.register_query(module::QueryMethodInfo {
            name: "bridge.NextSequenceNumbers",
            handler: Self::_query_next_sequence_numbers_handler,
        });
        methods.register_query(module::QueryMethodInfo {
            name: "bridge.Parameters",
            handler: Self::_query_parameters_handler,
        });
    }
}

impl<Accounts: modules::accounts::API> Module<Accounts> {
    fn init(ctx: &mut DispatchContext, genesis: &Genesis) {
        // Set genesis parameters.
        Self::set_params(ctx.runtime_state(), &genesis.parameters);
    }

    fn migrate(_ctx: &mut DispatchContext, _from: u32) -> bool {
        // No migrations currently supported.
        false
    }
}

impl<Accounts: modules::accounts::API> module::MigrationHandler for Module<Accounts> {
    type Genesis = Genesis;

    fn init_or_migrate(
        ctx: &mut DispatchContext,
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
