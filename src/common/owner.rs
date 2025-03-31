use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

pub enum OwnerKind {
    Keypair(Keypair),
    Pubkey(Pubkey),
}
pub struct Owner {
    inner: OwnerKind,
}
pub struct OwnerInfo {
    pub use_sol_balance: Option<bool>,
    pub fee_payer: Option<Pubkey>,
}

impl Owner {
    pub fn new(owner: OwnerKind) -> Self {
        Self { inner: owner }
    }
    pub fn pubkey(&self) -> Pubkey {
        match &self.inner {
            OwnerKind::Keypair(x) => x.pubkey(),
            OwnerKind::Pubkey(x) => *x,
        }
    }
    pub fn keypair(&self) -> Option<&Keypair> {
        match &self.inner {
            OwnerKind::Keypair(x) => Some(x),
            _ => None,
        }
    }
}
