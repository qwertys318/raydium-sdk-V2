use crate::common::pda::get_ata_address;
use crate::raydium::account::TokenAccount;
use solana_account_decoder_client_types::UiAccountData;
use solana_client::rpc_response::RpcKeyedAccount;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// @TODO return tokenAccountRawInfos also
pub fn parse_token_account_resp(
    owner: &Pubkey,
    sol_account_resp_opt: Option<solana_account::Account>,
    rpc_token_accounts: Vec<RpcKeyedAccount>,
) -> Vec<TokenAccount> {
    let mut token_accounts: Vec<TokenAccount> = rpc_token_accounts
        .into_iter()
        .map(|e| {
            let acc_pubkey = Pubkey::from_str(&e.pubkey).unwrap();
            let acc_owner = Pubkey::from_str(&e.account.owner).unwrap();
            let data = match e.account.data {
                UiAccountData::Json(x) => x,
                _ => unimplemented!(),
            };
            let mint = Pubkey::from_str(data.parsed["info"]["mint"].as_str().unwrap()).unwrap();
            TokenAccount {
                is_associated: Some(
                    get_ata_address(owner, &mint, Some(&acc_owner))
                        .0
                        .eq(&acc_pubkey),
                ),
                mint,
                amount: rug::Integer::from_str(
                    data.parsed["info"]["tokenAmount"]["amount"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap(),
                pubkey: Some(acc_pubkey),
                program_id: acc_owner,
                is_native: false,
            }
        })
        .collect();
    if let Some(sol_account_resp) = sol_account_resp_opt {
        token_accounts.push(TokenAccount {
            pubkey: None,
            mint: Pubkey::default(),
            is_associated: None,
            amount: rug::Integer::from_str(&sol_account_resp.lamports.to_string()).unwrap(),
            is_native: true,
            program_id: sol_account_resp.owner,
        });
    }
    token_accounts
}
