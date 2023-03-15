use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdResult, WasmMsg};

use crate::{msg::ExecuteMsg, state::RuleSet};

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }
}

pub fn is_valid_entropy(entropy: &str) -> bool {
    if entropy.len() == 128 {
        if let Ok(bytes) = hex::decode(entropy) {
            if bytes.len() == 64 {
                return true;
            }
        }
    }
    false
}

pub fn validate_ruleset(rule_set: &RuleSet) -> bool {
    if rule_set.zero.is_zero() || rule_set.one.is_zero() || rule_set.two.is_zero()
        || rule_set.three.is_zero() || rule_set.four.is_zero() || rule_set.five.is_zero()
        || rule_set.six.is_zero() {
            return false
    }

    let total_ratio: u128 = rule_set.zero.u128()
        + rule_set.one.u128()
        + rule_set.two.u128()
        + rule_set.three.u128()
        + rule_set.four.u128()
        + rule_set.five.u128()
        + rule_set.six.u128();
    if total_ratio != 129 {
        return false 
    }
    
    true
}
