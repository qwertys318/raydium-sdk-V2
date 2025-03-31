use crate::common::owner::Owner;
use crate::raydium::account::Account;
use solana_sdk::pubkey::Pubkey;

pub mod account;
pub mod clmm;
pub mod module_base;
pub mod tpe;

pub struct Raydium {
    pub account: Account,
}

impl Raydium {
    pub fn new(owner: Option<Owner>) -> Self {
        Self {
            account: Account::new(owner),
        }
    }
    pub fn owner_pubkey(&self) -> Result<Pubkey, String> {
        self.account.owner_pubkey()
    }
}
