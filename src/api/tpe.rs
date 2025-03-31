use solana_sdk::pubkey::Pubkey;

pub struct Base {
    pub lookup_table_account: Option<String>,
    pub vault: Vault,
    // program_id: Pubkey,
    // id: Pubkey,
}

pub struct Vault {
    pub a: Pubkey,
    pub b: Pubkey,
}
pub struct ClmmKeys {
    pub base: Base,
}
