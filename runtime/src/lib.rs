//! Bridge runtime.
use std::collections::{BTreeMap, BTreeSet};

use oasis_runtime_sdk::{
    self as sdk, core::common::version::Version, modules, types::token::Denomination,
};

/// Bridge runtime.
pub struct Runtime;

impl sdk::Runtime for Runtime {
    const VERSION: Version = Version::new(0, 0, 1);

    type Modules = (
        modules::accounts::Module,
        oasis_module_bridge::Module<modules::accounts::Module>,
    );

    fn genesis_state() -> <Self::Modules as sdk::module::MigrationHandler>::Genesis {
        (
            modules::accounts::Genesis {
                balances: {
                    let mut balances = BTreeMap::new();
                    // Alice.
                    balances.insert(sdk::testing::alice::address(), {
                        let mut denominations = BTreeMap::new();
                        denominations.insert(Denomination::NATIVE, 1_000_000.into());
                        denominations
                    });
                    // Bob.
                    balances.insert(sdk::testing::bob::address(), {
                        let mut denominations = BTreeMap::new();
                        denominations.insert(Denomination::NATIVE, 1_000_000.into());
                        denominations
                    });
                    // Charlie.
                    balances.insert(sdk::testing::charlie::address(), {
                        let mut denominations = BTreeMap::new();
                        denominations.insert(Denomination::NATIVE, 1_000_000.into());
                        denominations
                    });
                    balances
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
                            "oETH".into(),
                            "0000000000000000000000000000000000000000000000000000000000000000"
                                .into(),
                        );
                        rd
                    },
                    witnesses: vec![sdk::testing::bob::pk(), sdk::testing::charlie::pk()],
                    threshold: 2,
                },
            },
        )
    }
}
