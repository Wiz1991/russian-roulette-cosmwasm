use core::fmt;

use cosmwasm_std::BlockInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Expiration {
    AtHeight(u64),
    AtTime(u64),
    Never,
}

impl fmt::Display for Expiration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expiration::AtHeight(height) => write!(f, "Expiration height {}", height),
            Expiration::AtTime(time) => write!(f, "Expiration time {}", time),
            Expiration::Never => write!(f, "never"),
        }
    }
}

impl Default for Expiration {
    fn default() -> Self {
        Expiration::Never
    }
}

impl Expiration {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        match self {
            Expiration::AtHeight(height) => block.height >= *height,
            Expiration::AtTime(time) => block.time >= *time,
            Expiration::Never => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_expiration() {
        let block_h1000_t1000000 = BlockInfo {
            height: 1000,
            time: 1000000,
            chain_id: "test".to_string(),
        };

        let block_h2000_t2000000 = BlockInfo {
            height: 2000,
            time: 2000000,
            chain_id: "test".to_string(),
        };
        let exp_h1000 = Expiration::AtHeight(1000);
        let exp_t1000000 = Expiration::AtTime(1000000);
        let exp_h1500 = Expiration::AtHeight(1500);
        let exp_t1500000 = Expiration::AtTime(1500000);
        let exp_never = Expiration::default();

        assert!(exp_h1000.is_expired(&block_h1000_t1000000));
        assert!(!exp_h1500.is_expired(&block_h1000_t1000000));
        assert!(exp_h1500.is_expired(&block_h2000_t2000000));
        assert!(!exp_never.is_expired(&block_h2000_t2000000));
        assert!(exp_t1000000.is_expired(&block_h1000_t1000000));
        assert!(!exp_t1500000.is_expired(&block_h1000_t1000000));
        assert!(exp_t1500000.is_expired(&block_h2000_t2000000));
    }
}
