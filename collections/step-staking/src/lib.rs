use serde::{Deserialize, Serialize};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::AccountMeta,
    instruction::Instruction,
    message::{v0, Message, VersionedMessage},
    native_token::LAMPORTS_PER_SOL,
    pubkey,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    transaction::VersionedTransaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::ID as TOKEN_PROGRAM_ID;
use std::str::FromStr;
use znap::prelude::*;

use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest, transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use solana_client::rpc_client::RpcClient;
use borsh::{BorshSerialize, BorshDeserialize};

#[collection]
pub mod step_staking {

    use super::*;

    pub fn by_sol(ctx: Context<BySolAction>) -> Result<ActionTransaction> {
        let account_pubkey = Pubkey::from_str(&ctx.payload.account)
            .or_else(|_| Err(Error::from(ActionError::InvalidAccountPublicKey)))?;

        let program_id = pubkey!("Stk5NCWomVN3itaFjLu382u9ibb5jMSHEsh6CuhaGjB");

        let step_mint = pubkey!("StepAscQoEioFxxWGnh2sLBDFp9d8rvKz2Yp39iDpyT");
        let xstep_mint = pubkey!("xStpgUCss9piqeFUk2iLVcvJEGhAdJxJQuwLkXP555G");
        let native_mint = pubkey!("So11111111111111111111111111111111111111112");

        let step_associated_token_address =
            get_associated_token_address(&account_pubkey, &step_mint);
        let xstep_associated_token_address =
            get_associated_token_address(&account_pubkey, &xstep_mint);

        let amount = (ctx.query.amount * (LAMPORTS_PER_SOL as f32)) as u64;

        //// Jupiter Swap

        // let jupiter_swap_api_client =
        //     JupiterSwapApiClient::new("https://quote-api.jup.ag/v6".to_string());

        // let quote_request = QuoteRequest {
        //     amount,
        //     input_mint: native_mint,
        //     output_mint: step_mint,
        //     slippage_bps: 50,
        //     ..QuoteRequest::default()
        // };

        // let quote_response = jupiter_swap_api_client.quote(&quote_request).await.unwrap();

        // let swap_instructions = jupiter_swap_api_client
        //     .swap_instructions(&SwapRequest {
        //         user_public_key: account_pubkey,
        //         quote_response,
        //         config: TransactionConfig::default(),
        //     })
        //     .await
        //     .unwrap();

        //// STEP STAKING

        let seeds: &[&[u8]] = &[&step_mint.to_bytes()];
        let (vault_pubkey, vault_bump) = Pubkey::find_program_address(seeds, &program_id);

        let nonce: u8 = vault_bump;

        let args: InstructionArgs = InstructionArgs { nonce, amount };

        let instruction = Instruction::new_with_borsh(
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

        // TEST DATA

        // let to = pubkey!("HP1QHmCbaVmAv1N1pFALqugV6nLTq7UB673v3BNFZRr8");
        // let instruction = system_instruction::transfer(&account_pubkey, &to, amount);

        // VERSIONED VERSION

        // let rpc_url = "https://api.mainnet-beta.solana.com"; // Cambia a la red que prefieras
        // let client = RpcClient::new(rpc_url);

        // let blockhash = client.get_latest_blockhash().or_else(|_| Err(Error::from(ActionError::ProblemGettingLatestBlockhash)))?;

        // let messagev0 = VersionedMessage::V0(v0::Message::try_compile(
        //     &account_pubkey,
        //     &[instruction],
        //     &[],
        //     blockhash,
        // ).or_else(|_| Err(Error::from(ActionError::ProblemCreatingMessageV0)))?);

        // let transaction = VersionedTransaction::try_new(messagev0, &[]);

        // TRADITIONAL VERSION

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
#[query(amount: f32)]
pub struct BySolAction;

#[derive(ErrorCode)]
enum ActionError {
    #[error(msg = "Invalid account public key")]
    InvalidAccountPublicKey,
    // #[error(msg = "Problem getting latest blockhash")]
    // ProblemGettingLatestBlockhash,
    // #[error(msg = "Problem creating MessageV0")]
    // ProblemCreatingMessageV0,
}

// #[derive(Serialize, Deserialize)]
// pub struct InstructionArgs {
//     pub nonce: u8,
//     pub amount: u64,
// }

#[derive(BorshSerialize, BorshDeserialize)]
pub struct InstructionArgs {
    pub nonce: u8,
    pub amount: u64,
}