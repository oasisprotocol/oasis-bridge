//! Bridge runtime.
use std::collections::{BTreeMap, BTreeSet};

use oasis_runtime_sdk::{
    self as sdk, core::common::version::Version, modules, types::token::Denomination,
};

/// Bridge runtime.
pub struct Runtime;

impl sdk::Runtime for Runtime {
    const VERSION: Version = sdk::version_from_cargo!();

    type Modules = (
        modules::core::Module,
        modules::accounts::Module,
        oasis_module_bridge::Module<modules::accounts::Module>,
    );

    fn genesis_state() -> <Self::Modules as sdk::module::MigrationHandler>::Genesis {
        (
            modules::core::Genesis {
                parameters: modules::core::Parameters {
                    max_batch_gas: 10_000,
                    max_tx_signers: 8,
                    max_multisig_signers: 8,
                    ..Default::default()
                },
            },
            modules::accounts::Genesis {
                balances: {
                    let mut balances = BTreeMap::new();
                    // Alice.
                    balances.insert(sdk::testing::keys::alice::address(), {
                        let mut denominations = BTreeMap::new();
                        denominations.insert(Denomination::NATIVE, 1_000_000.into());
                        denominations
                    });
                    // Bob.
                    balances.insert(sdk::testing::keys::bob::address(), {
                        let mut denominations = BTreeMap::new();
                        denominations.insert(Denomination::NATIVE, 1_000_000.into());
                        denominations
                    });
                    // Charlie.
                    balances.insert(sdk::testing::keys::charlie::address(), {
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
            oasis_module_bridge::Genesis {
                parameters: oasis_module_bridge::Parameters {
                    local_denominations: {
                        let mut ld = BTreeSet::new();
                        ld.insert(Denomination::NATIVE);
                        ld
                    },
                    remote_denominations: {
                        let mut rd = BTreeMap::new();
                        rd.insert(
                            "oETH".parse().unwrap(),
                            "0000000000000000000000000000000000000000000000000000000000000000"
                                .into(),
                        );
                        rd
                    },
                    witnesses: vec![
                        sdk::testing::keys::bob::pk(),
                        sdk::testing::keys::charlie::pk(),
                        sdk::testing::keys::dave::pk(),
                    ],
                    threshold: 2,
                },
            },
        )
    }
}
