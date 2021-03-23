//! Bridge module types.
use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

const REMOTE_ADDRESS_SIZE: usize = 20;

/// Remote address-related error.
#[derive(Error, Debug)]
pub enum RemoteAddressError {
    #[error("malformed address")]
    MalformedAddress,
}

/// Remote address.
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RemoteAddress([u8; REMOTE_ADDRESS_SIZE]);

impl RemoteAddress {
    /// Tries to create a new remote address from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, RemoteAddressError> {
        if data.len() != REMOTE_ADDRESS_SIZE {
            return Err(RemoteAddressError::MalformedAddress);
        }

        let mut a = [0; REMOTE_ADDRESS_SIZE];
        a.copy_from_slice(data);

        Ok(RemoteAddress(a))
    }

    /// Tries to create a new remote address from hex-encoded string.
    pub fn from_hex(data: &str) -> Result<Self, RemoteAddressError> {
        let data =
            hex::decode(data.as_bytes()).map_err(|_| RemoteAddressError::MalformedAddress)?;
        RemoteAddress::from_bytes(&data)
    }
}

impl From<&str> for RemoteAddress {
    fn from(v: &str) -> RemoteAddress {
        RemoteAddress::from_hex(v).unwrap()
    }
}

impl fmt::LowerHex for RemoteAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in &self.0[..] {
            write!(f, "{:02x}", i)?;
        }
        Ok(())
    }
}

impl fmt::Debug for RemoteAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

impl fmt::Display for RemoteAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

impl serde::Serialize for RemoteAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let is_human_readable = serializer.is_human_readable();
        if is_human_readable {
            serializer.serialize_str(&hex::encode(&self.0))
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> serde::Deserialize<'de> for RemoteAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> serde::de::Visitor<'de> for BytesVisitor {
            type Value = RemoteAddress;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("bytes or string expected")
            }

            fn visit_str<E>(self, data: &str) -> Result<RemoteAddress, E>
            where
                E: serde::de::Error,
            {
                RemoteAddress::from_hex(data).map_err(serde::de::Error::custom)
            }

            fn visit_bytes<E>(self, data: &[u8]) -> Result<RemoteAddress, E>
            where
                E: serde::de::Error,
            {
                RemoteAddress::from_bytes(data).map_err(serde::de::Error::custom)
            }
        }

        if deserializer.is_human_readable() {
            Ok(deserializer.deserialize_string(BytesVisitor)?)
        } else {
            Ok(deserializer.deserialize_bytes(BytesVisitor)?)
        }
    }
}

/// Lock call.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    #[serde(rename = "target")]
    pub target: RemoteAddress,

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

    #[serde(rename = "target")]
    pub target: Address,

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
