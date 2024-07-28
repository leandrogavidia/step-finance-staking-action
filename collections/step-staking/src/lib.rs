use solana_sdk::{
    instruction::Instruction, 
    instruction::AccountMeta,
    message::Message, 
    native_token::LAMPORTS_PER_SOL, 
    pubkey::Pubkey, 
    transaction::Transaction,
    pubkey
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::ID as TOKEN_PROGRAM_ID;
use std::str::FromStr;
use znap::prelude::*; 
use serde::{
    Serialize,
    Deserialize 
};

#[collection] 
pub mod step_staking {
    use super::*;

    pub fn by_sol(ctx: Context< BySolAction>) -> Result<ActionTransaction> {
        let account_pubkey = Pubkey::from_str(&ctx.payload.account)
            .or_else(|_| Err(Error::from(ActionError::InvalidAccountPublicKey)))?;

        let program_id = pubkey!("Stk5NCWomVN3itaFjLu382u9ibb5jMSHEsh6CuhaGjB");

        let step_mint = pubkey!("StepAscQoEioFxxWGnh2sLBDFp9d8rvKz2Yp39iDpyT");
        let xstep_mint = pubkey!("xStpgUCss9piqeFUk2iLVcvJEGhAdJxJQuwLkXP555G");

        let step_associated_token_address = get_associated_token_address(&account_pubkey, &step_mint);
        let xstep_associated_token_address = get_associated_token_address(&account_pubkey, &xstep_mint);
        
        let seeds: &[&[u8]] = &[&step_mint.to_bytes()];
        let (vault_pubkey, vault_bump) = Pubkey::find_program_address(seeds, &program_id);
        
        let nonce: u8 = vault_bump;
        let amount = ctx.query.amount * LAMPORTS_PER_SOL;

        let args: InstructionArgs = InstructionArgs { nonce, amount };

        let instruction = Instruction::new_with_bincode(
            program_id,
            &args,
            vec![
                AccountMeta::new_readonly(step_mint, false),
                AccountMeta::new(xstep_mint, false),
                AccountMeta::new(step_associated_token_address, false),
                AccountMeta::new_readonly(account_pubkey, true),
                AccountMeta::new(vault_pubkey, false),
                AccountMeta::new(xstep_associated_token_address, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            ],
        ); 

        let message = Message::new(&[instruction], None);
        let transaction = Transaction::new_unsigned(message);

        Ok(ActionTransaction {
            transaction,
            message: Some("Stake Step".to_string()),
        })
    }
}

#[derive(Action)] 
#[action(
    icon = "https://raw.githubusercontent.com/leandrogavidia/files/main/xStep-01.png",
    title = "Stake Step by SOL",
    description = "You will stake",
    label = "Stake",
    link = {
        label = "Stake",
        href = "/api/by_sol?amount={amount}",
        parameter = { label = "Amount in SOL", name = "amount" }
    }
)]
#[query(amount: u64)]
pub struct BySolAction;

#[derive(ErrorCode)]
enum ActionError {
    #[error(msg = "Invalid account public key")]
    InvalidAccountPublicKey,
}

#[derive(Serialize, Deserialize)]
pub struct InstructionArgs {
    pub nonce: u8,
    pub amount: u64,
}