use bincode;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solana_sdk::{
    instruction::AccountMeta, instruction::Instruction, message::Message,
    native_token::LAMPORTS_PER_SOL, pubkey, pubkey::Pubkey, transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use spl_token::ID as TOKEN_PROGRAM_ID;
use std::str::FromStr;
use znap::prelude::*;

mod field_instruction;
mod field_pubkey;

#[collection]
pub mod step_staking {

    use super::*;

    pub fn by_sol(ctx: Context<BySolAction>) -> Result<ActionTransaction> {
        let account_pubkey = Pubkey::from_str(&ctx.payload.account)
            .or_else(|_| Err(Error::from(ActionError::InvalidAccountPublicKey)))?;

        let stake_program_id = pubkey!("Stk5NCWomVN3itaFjLu382u9ibb5jMSHEsh6CuhaGjB");

        let native_mint = pubkey!("So11111111111111111111111111111111111111112");
        let step_mint = pubkey!("StepAscQoEioFxxWGnh2sLBDFp9d8rvKz2Yp39iDpyT");
        let xstep_mint = pubkey!("xStpgUCss9piqeFUk2iLVcvJEGhAdJxJQuwLkXP555G");

        let step_associated_token_address =
            get_associated_token_address(&account_pubkey, &step_mint);
        let xstep_associated_token_address =
            get_associated_token_address(&account_pubkey, &xstep_mint);

        let seeds: &[&[u8]] = &[&step_mint.to_bytes()];
        let (vault_pubkey, vault_bump) = Pubkey::find_program_address(seeds, &stake_program_id);

        let nonce: u8 = vault_bump;

        let amount = (ctx.query.amount * (LAMPORTS_PER_SOL as f32)) as u64;

        // Create Step ATA instruction

        let create_xstep_ata_instruction = create_associated_token_account_idempotent(
            &account_pubkey,
            &account_pubkey,
            &step_mint,
            &TOKEN_PROGRAM_ID,
        );

        // Create xStep ATA instruction

        let create_step_ata_instruction = create_associated_token_account_idempotent(
            &account_pubkey,
            &account_pubkey,
            &xstep_mint,
            &TOKEN_PROGRAM_ID,
        );

        // Create swap Instruction

        let client = Client::new();
        let base_url = "https://quote-api.jup.ag/v6";

        let slippage_bps = "50";
        let max_accounts = "9";

        let quote_response = client
            .get(format!(
                "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&maxAccounts={}",
                base_url, native_mint, step_mint, amount, slippage_bps, max_accounts
            ))
            .send()
            .await
            .or_else(|_| Err(Error::from(ActionError::InternalServerError)))?
            .json::<QuoteResponse>()
            .await
            .or_else(|_| Err(Error::from(ActionError::InvalidResponseBody)))?;

        let step_amount = match quote_response.out_amount.parse::<u64>() {
            Ok(value) => value,
            Err(e) => {
                eprintln!("Error converting out_amount into u64: {}", e);
                0
            }
        };

        let swap_request = SwapRequest { quote_response, user_public_key: account_pubkey.to_string() };

        let swap_instructions = client
            .post(format!(
                "{}/swap-instructions",
                base_url
            ))
            .header("Accept", "application/json")
            .json(&swap_request)
            .send()
            .await
            .or_else(|_| Err(Error::from(ActionError::InternalServerError)))? 
            .json::<SwapInstructions>()
            .await
            .or_else(|_| Err(Error::from(ActionError::InvalidResponseBody)))?;

        let token_ledger_instruction = swap_instructions.token_ledger_instruction;
        let swap_compute_budget_instructions = swap_instructions.compute_budget_instructions;
        let setup_instructions = swap_instructions.setup_instructions;
        let swap_instruction = swap_instructions.swap_instruction;
        let cleanup_instruction = swap_instructions.cleanup_instruction;

        // Stake instruction

        let stake_args = StakeInstructionArgs { nonce, amount: step_amount };
        let stake_serialized_args =
            bincode::serialize(&stake_args).expect("Error serializing args");

        let mut stake_hasher = Sha256::new();
        stake_hasher.update(b"global:stake");
        let stake_result = stake_hasher.finalize();
        let stake_first_8_bytes = &stake_result[..8];

        let mut stake_data = Vec::new();
        stake_data.extend_from_slice(stake_first_8_bytes);
        stake_data.extend_from_slice(&stake_serialized_args);

        let stake_accounts = vec![
            AccountMeta::new_readonly(step_mint, false),
            AccountMeta::new(xstep_mint, false),
            AccountMeta::new(step_associated_token_address, false),
            AccountMeta::new_readonly(account_pubkey, true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(xstep_associated_token_address, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ];

        let stake_instruction =
            Instruction::new_with_bytes(stake_program_id, &stake_data, stake_accounts);

        // Send transaction

        let mut instructions = vec![
            create_step_ata_instruction,
            create_xstep_ata_instruction,
        ]; 

        if let Some(instruction) = token_ledger_instruction {
            instructions.push(instruction);
        }

        instructions.extend_from_slice(&swap_compute_budget_instructions);
        instructions.extend_from_slice(&setup_instructions);
        instructions.push(swap_instruction);

        if let Some(instruction) = cleanup_instruction {
            instructions.push(instruction);
        }

        instructions.push(stake_instruction);

        let message = Message::new(
            &instructions,
            None,
        );

        let transaction = Transaction::new_unsigned(message);

        Ok(ActionTransaction {
            transaction,
            message: Some("Stake Step".to_string()),
        })
    }
}

#[derive(Action)]
#[action(
    icon = "https://raw.githubusercontent.com/leandrogavidia/files/main/blink-step-finance-staking-by-sol.jpg",
    title = "Stake xStep",
    description = "From SOL to xStep",
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
    #[error(msg = "Internal server error")]
    InternalServerError,
    #[error(msg = "Invalid response body")]
    InvalidResponseBody,
}

#[derive(Serialize, Deserialize)]
pub struct InitializeXstepInstructionArgs {
    pub nonce: u8,
}

#[derive(Serialize, Deserialize)]
pub struct StakeInstructionArgs {
    pub nonce: u8,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwapInfo {
    amm_key: String,
    label: String,
    input_mint: String,
    output_mint: String,
    in_amount: String,
    out_amount: String,
    fee_amount: String,
    fee_mint: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Route {
    swap_info: SwapInfo,
    percent: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteResponse {
    input_mint: String,
    in_amount: String,
    output_mint: String,
    out_amount: String,
    other_amount_threshold: String,
    swap_mode: String,
    slippage_bps: u32,
    platform_fee: Option<u32>,
    price_impact_pct: String,
    route_plan: Vec<Route>,
    context_slot: u64,
    time_taken: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwapRequest {
    quote_response: QuoteResponse,
    user_public_key: String
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInstructions {
    #[serde(with = "field_instruction::option_instruction")]
    pub token_ledger_instruction: Option<Instruction>,
    #[serde(with = "field_instruction::vec_instruction")]
    pub compute_budget_instructions: Vec<Instruction>,
    #[serde(with = "field_instruction::vec_instruction")]
    pub setup_instructions: Vec<Instruction>,
    #[serde(with = "field_instruction::instruction")]
    pub swap_instruction: Instruction,
    #[serde(with = "field_instruction::option_instruction")]
    pub cleanup_instruction: Option<Instruction>,
    #[serde(with = "field_pubkey::vec")]
    pub address_lookup_table_addresses: Vec<Pubkey>,
    pub prioritization_fee_lamports: u64,
}