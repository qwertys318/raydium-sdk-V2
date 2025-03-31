use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn get_ata_address(owner: &Pubkey, mint: &Pubkey, program_id: Option<&Pubkey>) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            owner.to_bytes().as_slice(),
            program_id
                .unwrap_or_else(|| &spl_token::ID)
                .to_bytes()
                .as_slice(),
            mint.to_bytes().as_slice(),
        ],
        &Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap(),
    )
}
