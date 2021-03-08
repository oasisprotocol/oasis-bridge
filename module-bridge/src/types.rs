//! Bridge module types.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use oasis_runtime_sdk::{
    core::common::{cbor, crypto::hash::Hash},
    crypto::signature::Signature,
    types::{address::Address, token},
};

/// Remote denomination identifier.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteDenomination(#[serde(with = "serde_bytes")] Vec<u8>);

impl RemoteDenomination {
    /// Maximum length of a remote denomination.
    pub const MAX_LENGTH: usize = 32;

    // TODO: Enforce maximum length during deserialization.
}

impl From<&str> for RemoteDenomination {
    fn from(v: &str) -> RemoteDenomination {
        RemoteDenomination(hex::decode(v.as_bytes()).unwrap())
    }
}

/// Lock call.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    #[serde(rename = "amount")]
    pub amount: token::BaseUnits,
}

/// Lock call results.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockResult {
    #[serde(rename = "id")]
    pub id: u64,
}

/// Witness event call.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Witness {
    #[serde(rename = "id")]
    pub id: u64,

    #[serde(rename = "sig")]
    pub signature: Signature,
}

/// Release call.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    #[serde(rename = "id")]
    pub id: u64,

    #[serde(rename = "owner")]
    pub owner: Address,

    #[serde(rename = "amount")]
    pub amount: token::BaseUnits,
}

/// Operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Operation {
    #[serde(rename = "lock")]
    Lock(Lock),

    #[serde(rename = "release")]
    Release(Release),
}

/// A unique operation identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OperationId(Hash);

impl From<&Operation> for OperationId {
    fn from(op: &Operation) -> OperationId {
        OperationId(Hash::digest_bytes(&cbor::to_vec(op)))
    }
}

/// Witness signatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessSignatures {
    #[serde(rename = "id")]
    pub id: u64,

    #[serde(rename = "op")]
    pub op: Operation,

    #[serde(rename = "wits")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub witnesses: Vec<u16>,

    #[serde(rename = "sigs")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<Signature>,
}

impl WitnessSignatures {
    /// Create a new empty set of witness signatures.
    pub fn new(id: u64, op: Operation) -> Self {
        Self {
            id,
            op,
            witnesses: Vec::new(),
            signatures: Vec::new(),
        }
    }
}

/// Incoming witness signatures.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomingWitnessSignatures {
    #[serde(rename = "ops")]
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub ops: BTreeMap<OperationId, WitnessSignatures>,

    #[serde(rename = "wits")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub witnesses: Vec<u16>,
}

/// Next event sequence numbers.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NextSequenceNumbers {
    #[serde(rename = "in")]
    pub incoming: u64,

    #[serde(rename = "out")]
    pub outgoing: u64,
}
