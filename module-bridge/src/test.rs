//! Tests for the bridge module.
use std::collections::{BTreeMap, BTreeSet};

use oasis_runtime_sdk::{
    context::{BatchContext, Context},
    core::common::cbor,
    crypto::signature::PublicKey,
    module::MigrationHandler,
    modules::{
        accounts::{self, Module as Accounts, API as AccountsAPI},
        core,
    },
    testing::{keys, mock},
    types::{
        token::{BaseUnits, Denomination},
        transaction,
    },
};

use super::{types::*, Error, Genesis, Parameters, ADDRESS_LOCKED_FUNDS};

type Bridge = super::Module<Accounts>;

fn init_accounts<C: Context>(ctx: &mut C) {
    Accounts::init_or_migrate(
        ctx,
        &mut core::types::Metadata::default(),
        &accounts::Genesis {
            balances: {
                let mut balances = BTreeMap::new();
                // Alice.
                balances.insert(keys::alice::address(), {
                    let mut denominations = BTreeMap::new();
                    denominations.insert(Denomination::NATIVE, 1_000_000.into());
                    denominations
                });
                // Bob.
                balances.insert(keys::bob::address(), {
                    let mut denominations = BTreeMap::new();
                    denominations.insert(Denomination::NATIVE, 1_000_000.into());
                    denominations
                });
                // Charlie.
                balances.insert(keys::charlie::address(), {
                    let mut denominations = BTreeMap::new();
                    denominations.insert(Denomination::NATIVE, 1_000_000.into());
                    denominations
                });
                balances
            },
            total_supplies: {
                let mut total_supplies = BTreeMap::new();
                total_supplies.insert(Denomination::NATIVE, 3_000_000.into());
                total_supplies
            },
            ..Default::default()
        },
    );
}

fn init_bridge<C: Context>(ctx: &mut C) -> Parameters {
    init_bridge_ex(ctx, vec![keys::bob::pk(), keys::charlie::pk()])
}

fn init_bridge_ex<C: Context>(ctx: &mut C, witnesses: Vec<PublicKey>) -> Parameters {
    let parameters = Parameters {
        local_denominations: {
            let mut ld = BTreeSet::new();
            ld.insert(Denomination::NATIVE);
            ld
        },
        remote_denominations: {
            let mut rd = BTreeMap::new();
            rd.insert(
                "oETH".parse().unwrap(),
                "0000000000000000000000000000000000000000000000000000000000000000".into(),
            );
            rd
        },
        witnesses,
        threshold: 2,
    };

    Bridge::init_or_migrate(
        ctx,
        &mut core::types::Metadata::default(),
        &Genesis {
            parameters: parameters.clone(),
            ..Default::default()
        },
    );

    parameters
}

#[test]
fn test_outgoing_basic() {
    let mut mock = mock::Mock::default();
    let mut ctx = mock.create_ctx();

    init_accounts(&mut ctx);
    init_bridge(&mut ctx);

    // User Alice locks an amount.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Lock".to_owned(),
            body: cbor::to_value(Lock {
                target: "0000000000000000000000000000000000000000".into(),
                amount: BaseUnits::new(1_000.into(), Denomination::NATIVE),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::alice::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        Bridge::tx_lock(&mut tx_ctx, cbor::from_value(call.body).unwrap())
            .expect("lock should succeed");

        // Check source account balances.
        let bals = Accounts::get_balances(tx_ctx.runtime_state(), keys::alice::address())
            .expect("get_balances should succeed");
        assert_eq!(
            bals.balances[&Denomination::NATIVE],
            999_000.into(),
            "balance in source account should be correct"
        );
        assert_eq!(
            bals.balances.len(),
            1,
            "there should only be one denomination"
        );

        // Check bridge module account balances.
        let bals = Accounts::get_balances(tx_ctx.runtime_state(), *ADDRESS_LOCKED_FUNDS)
            .expect("get_balances should succeed");
        assert_eq!(
            bals.balances[&Denomination::NATIVE],
            1_000.into(),
            "balance in destination account should be correct"
        );
        assert_eq!(
            bals.balances.len(),
            1,
            "there should only be one denomination"
        );

        let (_tags, _messages) = tx_ctx.commit();
    });

    // Witness Bob witnesses the local event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Witness".to_owned(),
            body: cbor::to_value(Witness {
                id: 0,
                signature: vec![].into(),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::bob::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        Bridge::tx_witness(&mut tx_ctx, cbor::from_value(call.body.clone()).unwrap())
            .expect("witness should succeed");

        // Duplicate witness should be rejected.
        let result = Bridge::tx_witness(&mut tx_ctx, cbor::from_value(call.body).unwrap());
        assert!(matches!(result, Err(Error::AlreadySubmittedSignature)));

        let (_tags, _messages) = tx_ctx.commit();
    });

    // Witness Charlie witnesses the local event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Witness".to_owned(),
            body: cbor::to_value(Witness {
                id: 0,
                signature: vec![].into(),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::charlie::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        Bridge::tx_witness(&mut tx_ctx, cbor::from_value(call.body).unwrap())
            .expect("witness should succeed");

        let (_tags, _messages) = tx_ctx.commit();
    });
}

#[test]
fn test_outgoing_fail_not_authorized() {
    let mut mock = mock::Mock::default();
    let mut ctx = mock.create_ctx();

    init_accounts(&mut ctx);
    init_bridge(&mut ctx);

    // Witness Alice (who is not a witness) witnesses an event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Witness".to_owned(),
            body: cbor::to_value(Witness {
                id: 0,
                signature: vec![].into(),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::alice::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        let result = Bridge::tx_witness(&mut tx_ctx, cbor::from_value(call.body).unwrap());
        assert!(matches!(result, Err(Error::NotAuthorized)));
    });
}

#[test]
fn test_outgoing_fail_invalid_sequence() {
    let mut mock = mock::Mock::default();
    let mut ctx = mock.create_ctx();

    init_accounts(&mut ctx);
    init_bridge(&mut ctx);

    // Witness Bob witnesses an invalid event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Witness".to_owned(),
            body: cbor::to_value(Witness {
                id: 0,
                signature: vec![].into(),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::bob::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        let result = Bridge::tx_witness(&mut tx_ctx, cbor::from_value(call.body).unwrap());
        assert!(matches!(result, Err(Error::InvalidSequenceNumber)));
    });
}

#[test]
fn test_incoming_basic() {
    let mut mock = mock::Mock::default();
    let mut ctx = mock.create_ctx();

    init_accounts(&mut ctx);
    init_bridge(&mut ctx);

    // Witness Bob witnesses the remote event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Release".to_owned(),
            body: cbor::to_value(Release {
                id: 0,
                target: keys::alice::address(),
                amount: BaseUnits::new(1_000.into(), "oETH".parse().unwrap()),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::bob::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        Bridge::tx_release(&mut tx_ctx, cbor::from_value(call.body.clone()).unwrap())
            .expect("release should succeed");

        // Check source account balances.
        let bals = Accounts::get_balances(tx_ctx.runtime_state(), keys::alice::address())
            .expect("get_balances should succeed");
        assert_eq!(
            bals.balances[&Denomination::NATIVE],
            1_000_000.into(),
            "balance in target account should be unchanged"
        );
        assert_eq!(
            bals.balances.len(),
            1,
            "there should only be one denomination"
        );

        // Check bridge module account balances.
        let bals = Accounts::get_balances(tx_ctx.runtime_state(), *ADDRESS_LOCKED_FUNDS)
            .expect("get_balances should succeed");
        assert!(
            bals.balances.is_empty(),
            "there should be no balances in the bridge module account"
        );

        // Duplicate releases should be rejected.
        let result = Bridge::tx_release(&mut tx_ctx, cbor::from_value(call.body).unwrap());
        assert!(matches!(result, Err(Error::AlreadySubmittedSignature)));

        let (_tags, _messages) = tx_ctx.commit();
    });

    // Witness Charlie witnesses the remote event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Release".to_owned(),
            body: cbor::to_value(Release {
                id: 0,
                target: keys::alice::address(),
                amount: BaseUnits::new(1_000.into(), "oETH".parse().unwrap()),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::charlie::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        Bridge::tx_release(&mut tx_ctx, cbor::from_value(call.body).unwrap())
            .expect("release should succeed");

        // Check source account balances.
        let bals = Accounts::get_balances(tx_ctx.runtime_state(), keys::alice::address())
            .expect("get_balances should succeed");
        assert_eq!(
            bals.balances[&Denomination::NATIVE],
            1_000_000.into(),
            "native token balance in target account should be unchanged"
        );
        assert_eq!(
            bals.balances[&"oETH".parse().unwrap()],
            1_000.into(),
            "tokens should have been minted"
        );
        assert_eq!(
            bals.balances.len(),
            2,
            "there should now be two denominations"
        );

        // Check bridge module account balances.
        let bals = Accounts::get_balances(tx_ctx.runtime_state(), *ADDRESS_LOCKED_FUNDS)
            .expect("get_balances should succeed");
        assert_eq!(
            bals.balances[&"oETH".parse().unwrap()],
            0.into(),
            "no minted tokens should remain in the bridge account"
        );
        assert_eq!(
            bals.balances.len(),
            1,
            "there should be a single denomination"
        );

        let (_tags, _messages) = tx_ctx.commit();
    });
}

#[test]
fn test_incoming_fail_invalid_sequence() {
    let mut mock = mock::Mock::default();
    let mut ctx = mock.create_ctx();

    init_accounts(&mut ctx);
    init_bridge(&mut ctx);

    // Witness Bob witnesses the remote event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Release".to_owned(),
            body: cbor::to_value(Release {
                id: 1, // Invalid sequence as it should be 0.
                target: keys::alice::address(),
                amount: BaseUnits::new(1_000.into(), "oETH".parse().unwrap()),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::bob::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        let result = Bridge::tx_release(&mut tx_ctx, cbor::from_value(call.body).unwrap());
        assert!(matches!(result, Err(Error::InvalidSequenceNumber)));
    });
}

#[test]
fn test_incoming_fail_divergence() {
    let mut mock = mock::Mock::default();
    let mut ctx = mock.create_ctx();

    init_accounts(&mut ctx);
    init_bridge_ex(
        &mut ctx,
        vec![keys::alice::pk(), keys::bob::pk(), keys::charlie::pk()],
    );

    // Witness Bob witnesses the remote event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Release".to_owned(),
            body: cbor::to_value(Release {
                id: 0,
                target: keys::alice::address(),
                amount: BaseUnits::new(1_000.into(), "oETH".parse().unwrap()),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::bob::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        Bridge::tx_release(&mut tx_ctx, cbor::from_value(call.body.clone()).unwrap())
            .expect("release should succeed");

        let (_tags, _messages) = tx_ctx.commit();
    });

    // Witness Charlie witnesses a diverging event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Release".to_owned(),
            body: cbor::to_value(Release {
                id: 0,
                target: keys::alice::address(),
                amount: BaseUnits::new(2_000.into(), "oETH".parse().unwrap()),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::charlie::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        Bridge::tx_release(&mut tx_ctx, cbor::from_value(call.body.clone()).unwrap())
            .expect("release should succeed");

        let (_tags, _messages) = tx_ctx.commit();
    });

    // Check source account balances.
    let bals = Accounts::get_balances(ctx.runtime_state(), keys::alice::address())
        .expect("get_balances should succeed");
    assert_eq!(
        bals.balances[&Denomination::NATIVE],
        1_000_000.into(),
        "balance in target account should be unchanged"
    );
    assert_eq!(
        bals.balances.len(),
        1,
        "there should only be one denomination"
    );

    // Check bridge module account balances.
    let bals = Accounts::get_balances(ctx.runtime_state(), *ADDRESS_LOCKED_FUNDS)
        .expect("get_balances should succeed");
    assert!(
        bals.balances.is_empty(),
        "there should be no balances in the bridge module account"
    );

    // Witness Alice witnesses a correct event.
    let tx = transaction::Transaction {
        version: 1,
        call: transaction::Call {
            method: "bridge.Release".to_owned(),
            body: cbor::to_value(Release {
                id: 0,
                target: keys::alice::address(),
                amount: BaseUnits::new(1_000.into(), "oETH".parse().unwrap()),
            }),
        },
        auth_info: transaction::AuthInfo {
            signer_info: vec![transaction::SignerInfo::new(keys::alice::pk(), 0)],
            fee: transaction::Fee {
                amount: Default::default(),
                gas: 1000,
            },
        },
    };
    ctx.with_tx(tx, |mut tx_ctx, call| {
        Bridge::tx_release(&mut tx_ctx, cbor::from_value(call.body.clone()).unwrap())
            .expect("release should succeed");

        let (_tags, _messages) = tx_ctx.commit();
    });

    // Check source account balances.
    let bals = Accounts::get_balances(ctx.runtime_state(), keys::alice::address())
        .expect("get_balances should succeed");
    assert_eq!(
        bals.balances[&Denomination::NATIVE],
        1_000_000.into(),
        "native token balance in target account should be unchanged"
    );
    assert_eq!(
        bals.balances[&"oETH".parse().unwrap()],
        1_000.into(),
        "tokens should have been minted"
    );
    assert_eq!(
        bals.balances.len(),
        2,
        "there should now be two denominations"
    );

    // Check bridge module account balances.
    let bals = Accounts::get_balances(ctx.runtime_state(), *ADDRESS_LOCKED_FUNDS)
        .expect("get_balances should succeed");
    assert_eq!(
        bals.balances[&"oETH".parse().unwrap()],
        0.into(),
        "no minted tokens should remain in the bridge account"
    );
    assert_eq!(
        bals.balances.len(),
        1,
        "there should be a single denomination"
    );
}

#[test]
fn test_query_parameters() {
    let mut mock = mock::Mock::default();
    let mut ctx = mock.create_ctx();

    init_accounts(&mut ctx);
    let genesis_params = init_bridge(&mut ctx);

    let params = Bridge::query_parameters(&mut ctx, ()).expect("parameters query should succeed");
    // XXX: Compare params directly once oasis-sdk#68 is merged and version is bumped.
    assert_eq!(
        params.threshold, genesis_params.threshold,
        "parameter query should return correct results"
    );
    assert_eq!(
        params.local_denominations, genesis_params.local_denominations,
        "parameter query should return correct results"
    );
    assert_eq!(
        params.remote_denominations, genesis_params.remote_denominations,
        "parameter query should return correct results"
    );
}
