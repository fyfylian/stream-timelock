use anchor_lang::prelude::*;
use anchor_lang::solana_program::{system_program, program::invoke, sysvar::clock::Clock};
use anchor_spl::token::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod timelock {
    use super::*;

    // #[access_control(CreateVesting::accounts(&ctx, nonce))]
    pub fn create(
        ctx: Context<Create>,
        beneficiary: Pubkey,
        deposited_amount: u64,
        // nonce: u8,
        start: u64,
        end: u64,
        // period_count: u64,
        //   realizor: Option<Realizor>,
    ) -> ProgramResult {
        if deposited_amount <= 0 {
            return Err(VestingError::ZeroAmount.into());
        }

        // if !is_valid_schedule(start_ts, end_ts, period_count) { //todo create this method
        //     return Err(ErrorCode::InvalidSchedule.into());
        // }

        //accounts check

        let pda = &mut ctx.accounts.pda;
        pda.depositor = ctx.accounts.depositor.key();
        pda.beneficiary = beneficiary;
        pda.mint = ctx.accounts.pda_token_acc.mint;
        pda.pda_token_acc = ctx.accounts.pda_token_acc.key();
        pda.deposited_amount = deposited_amount;
        pda.start = start;
        pda.end = end;
        //pda.created_ts = ctx.accounts.clock.unix_timestamp;
        pda.withdrawn = 0;
        pda.unlocked = 0;

        // transfer(ctx.accounts.into(), deposited_amount)?;

        let ix = spl_token::instruction::transfer(
            &spl_token::ID,
            &ctx.accounts.depositor_token_acc.key(),
            &ctx.accounts.pda_token_acc.key(),
            &ctx.accounts.depositor.key(),
            &[],
            deposited_amount,
        )?;
        invoke(
            &ix,
            &[
                ctx.accounts.depositor_token_acc.to_account_info().clone(),
                ctx.accounts.pda_token_acc.to_account_info().clone(),
                ctx.accounts.depositor.to_account_info().clone(),
                ctx.accounts.token_program.to_account_info().clone(),
            ],
        )
    }


    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> ProgramResult {
        let pda = &mut ctx.accounts.pda;

        //todo might be unnecessary due to macro attribute check in the context
        if ctx.accounts.beneficiary.key() != pda.beneficiary
            || ctx.accounts.pda_token_acc.key() != pda.pda_token_acc { Err(ProgramError::InvalidAccountData)? }

        let available = available(&pda, Clock::get()?.unix_timestamp as u64);
        if amount > available { Err(ProgramError::AccountBorrowFailed)? }

        //will this persist if the token transfer fail?
        pda.withdrawn = pda.withdrawn.checked_add(amount).unwrap();

        transfer_tokens(&ctx.accounts.pda_token_acc.to_account_info(),
                        &ctx.accounts.beneficiary_token_acc.to_account_info(),
                        &pda.to_account_info(),
                        &ctx.accounts.token_program,
                        amount)
    }

    pub fn cancel(ctx: Context<Cancel>) -> ProgramResult {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(init, payer = depositor)]
    pub pda: Account<'info, VestingContract>,
    #[account(mut)]
    pub pda_token_acc: Account<'info, TokenAccount>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    #[account(mut)]
    pub depositor_token_acc: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, signer, has_one = beneficiary, has_one = pda_token_acc)]
    pub pda: Account<'info, VestingContract>,
    #[account(mut)]
    pub pda_token_acc: Account<'info, TokenAccount>,
    #[account(mut, constraint = beneficiary_token_acc.owner == *beneficiary.key)]
    pub beneficiary_token_acc: Account<'info, TokenAccount>,
    pub beneficiary: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut, has_one = depositor, has_one = beneficiary, has_one = pda_token_acc)]
    pub pda: Account<'info, VestingContract>,
    // todo macros
    pub depositor: Signer<'info>,
    pub beneficiary: UncheckedAccount<'info>,
    pub pda_token_acc: Account<'info, TokenAccount>,
}

#[account]
#[derive(Default)]
pub struct VestingContract {
    /// Issuer of the Vesting Contract
    pub depositor: Pubkey,
    /// Beneficiary of the Vesting Contract
    pub beneficiary: Pubkey,
    /// Mint of a specific SPL token being vested
    pub mint: Pubkey,
    /// SPL token address where tokens are deposited
    pub pda_token_acc: Pubkey,
    /// Timestamp — when contract begins
    pub start: u64,
    /// Timestamp — when tokens are fully vested
    pub end: u64,
    /// Time step (period) per which the vesting occurs
    // pub unlock_step: u64,
    /// Original amount deposited in the contract.
    pub deposited_amount: u64,
    //in smallest possible token denomination (e.g. lamports for SOL)
    /// Amount of the tokens unlocked (vested)
    pub unlocked: u64,
    //can be derived from `deposited_amount` and `withdrawn`
    /// Amount of the tokens withdrawn from the contract
    pub withdrawn: u64,
    //   /// (optional) Vesting contract "cliff" timestamp
    // pub cliff: Option<u64>,
    //   /// (optional) Amount unlocked at the "cliff" timestamp
    // pub cliff_amount: Option<u64>,
    // pub fee: Option<FeeTier>, //todo not used atm, but test its behavior
    // todo do we need nonce and/or seed?
    // todo need additional data?
}

#[error]
pub enum VestingError {
    #[msg("Invalid schedule.")]
    InvalidSchedule,
    #[msg("Amount must be greater than 0.")]
    ZeroAmount,
}

//todo move to a separate file
pub fn available(contract: &VestingContract, now: u64) -> u64 {
    if contract.start < now { return 0; }

    // (now - start) / (end - start) * deposited_amount - withdrawn
    //todo: floats are imprecise, use integer division (requires witty solutions, adds complexity)
    let percent_unlocked = ((now - contract.start) as f64) / ((contract.end - contract.start) as f64);

    std::cmp::min(((percent_unlocked * contract.deposited_amount as f64) as u64)
                      .checked_sub(contract.withdrawn).unwrap(),
                  contract.deposited_amount)
}

pub fn transfer_tokens<'a>(from: &AccountInfo<'a>, to: &AccountInfo<'a>, auth: &AccountInfo<'a>, token_program: &Program<'a, Token>, amount: u64) -> ProgramResult {
    let ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &from.key(),
        &to.key(),
        &auth.key(),
        &[],
        amount,
    )?;
    invoke(
        &ix,
        &[
            //account order is important!
            from.to_account_info(),
            to.to_account_info(),
            auth.to_account_info(),
            token_program.to_account_info(),
        ],
    )
}
