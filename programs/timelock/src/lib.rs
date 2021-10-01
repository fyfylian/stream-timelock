use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, sysvar::clock::Clock};
use anchor_spl::token::*;
use anchor_lang::solana_program::program::invoke_signed;


declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod timelock {
    use super::*;

    pub fn create(
        ctx: Context<Create>,
        beneficiary: Pubkey,
        deposited_amount: u64,
        start: u64,
        end: u64,
        period: u64,
        nonce: u8,
        cliff: Option<u64>,
        cliff_amount: Option<u64>,
    ) -> ProgramResult {
        let now = Clock::get()?.unix_timestamp as u64;
        let a = ctx.accounts;
        let pda = &mut a.pda;

        if deposited_amount <= 0 {
            return Err(VestingError::ZeroAmount.into());
        }

        if !is_valid_schedule(start, end, now, cliff) {
            return Err(VestingError::InvalidSchedule.into());
        }

        //todo additional accounts check?

        pda.depositor = a.depositor.key();
        pda.beneficiary = beneficiary;
        pda.mint = a.pda_token_acc.mint;
        pda.pda_token_acc = a.pda_token_acc.key();
        pda.deposited_amount = deposited_amount;
        pda.start = start;
        pda.end = end;
        pda.period = period;
        pda.nonce = nonce;
        pda.cliff = cliff;
        pda.cliff_amount = cliff_amount;
        pda.withdrawn = 0;

        let seeds = [
            pda.to_account_info().key.as_ref(),
            &[pda.nonce],
        ];
        let signer = &[&seeds[..]];
        transfer_tokens(
            &a.depositor_token_acc.to_account_info(),
            &a.pda_token_acc.to_account_info(),
            &a.depositor.to_account_info(),
            &a.token_program,
            signer,
            deposited_amount,
        )
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> ProgramResult {
        let now = Clock::get()?.unix_timestamp as u64;
        let a = ctx.accounts;
        let pda = &mut a.pda;

        let available = available(&pda, now);
        if amount > available { Err(ProgramError::InvalidArgument)? };

        pda.withdrawn = pda.withdrawn.checked_add(amount).unwrap();

        let seeds = [
            pda.to_account_info().key.as_ref(),
            &[pda.nonce],
        ];
        let signer = &[&seeds[..]];
        transfer_tokens(&a.pda_token_acc.to_account_info(),
                        &a.beneficiary_token_acc.to_account_info(),
                        &a.pda_signer.to_account_info(),
                        &a.token_program,
                        signer,
                        amount)
    }

    pub fn cancel(ctx: Context<Cancel>) -> ProgramResult {
        let now = Clock::get()?.unix_timestamp as u64;
        let a = ctx.accounts;
        let pda = &mut a.pda;

        let available = available(&pda, now);
        msg!("start: {}, now: {}, end {}, now - start {}, end - now {}",
        pda.start, now, pda.end, now - pda.start, pda.end - now);
        msg!("total: {}, available: {}, returned: {}",
            pda.deposited_amount, available, pda.deposited_amount - available);

        let seeds = [
            pda.to_account_info().key.as_ref(),
            &[pda.nonce],
        ];
        let signer = &[&seeds[..]];

        transfer_tokens(&a.pda_token_acc.to_account_info(),
                        &a.beneficiary_token_acc.to_account_info(),
                        &a.pda_signer.to_account_info(),
                        &a.token_program,
                        signer,
                        available - 11, //todo wtf fix this
        );

        //send the rest back to the depositor
        transfer_tokens(&a.pda_token_acc.to_account_info(),
                        &a.depositor_token_acc.to_account_info(),
                        &a.pda_signer.to_account_info(),
                        &a.token_program,
                        signer,
                        pda.deposited_amount - available,
        );
        pda.withdrawn = pda.deposited_amount;
        Ok(())
    }

    pub fn transfer(ctx: Context<Transfer>) -> ProgramResult {
        let pda = &mut ctx.accounts.pda;
        pda.beneficiary = ctx.accounts.new_beneficiary.key();

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
    pub system_program: Program<'info, System>, //needed for acc init
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, has_one = beneficiary, has_one = pda_token_acc)]
    pub pda: Account<'info, VestingContract>,
    #[account(mut)]
    pub pda_token_acc: Account<'info, TokenAccount>,
    #[account(seeds = [pda.to_account_info().key.as_ref()], bump = pda.nonce)]
    pub pda_signer: UncheckedAccount<'info>,
    #[account(mut, constraint = beneficiary_token_acc.owner == * beneficiary.key)]
    pub beneficiary_token_acc: Account<'info, TokenAccount>,
    pub beneficiary: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut, has_one = depositor, has_one = beneficiary, has_one = pda_token_acc)]
    pub pda: Account<'info, VestingContract>,
    #[account(mut)]
    pub pda_token_acc: Account<'info, TokenAccount>,
    #[account(seeds = [pda.to_account_info().key.as_ref()], bump = pda.nonce)]
    pub pda_signer: UncheckedAccount<'info>,
    pub depositor: Signer<'info>,
    #[account(mut)]
    pub depositor_token_acc: Account<'info, TokenAccount>,
    pub beneficiary: UncheckedAccount<'info>,
    #[account(mut, constraint = beneficiary_token_acc.owner == * beneficiary.key)]
    pub beneficiary_token_acc: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut, has_one = beneficiary)]
    pub pda: Account<'info, VestingContract>,
    pub beneficiary: Signer<'info>,
    pub new_beneficiary: UncheckedAccount<'info>,
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
    /// Timestamp — vesting begins
    pub start: u64,
    /// Timestamp — tokens are fully vested
    pub end: u64,
    /// Time step (period) per which the vesting occurs
    pub period: u64,
    /// Original amount deposited in the contract.
    pub deposited_amount: u64,
    //in smallest possible token denomination (e.g. lamports for SOL)
    /// Amount of the tokens withdrawn from the contract
    pub withdrawn: u64,
    /// Signer nonce.
    pub nonce: u8,
    /// (optional) Vesting contract "cliff" timestamp
    pub cliff: Option<u64>,
    /// (optional) Amount unlocked at the "cliff" timestamp
    pub cliff_amount: Option<u64>,
    pub fee: Option<FeeTier>, //todo not used atm, but let's test it
    // todo think about need additional data in order to keep the contract structure
}

#[error]
pub enum VestingError {
    #[msg("Invalid schedule.")]
    InvalidSchedule,
    #[msg("Amount must be greater than 0.")]
    ZeroAmount,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum FeeTier {
    Free,
    LowestFee,
    LowFee,
    NormalFee,
}

//todo move to a separate file
pub fn available(contract: &VestingContract, now: u64) -> u64 {
    if contract.start > now || contract.cliff.unwrap() > now { return 0; }
    if now >= contract.end { return contract.deposited_amount; }

    //set default values if not set
    let cliff = contract.cliff.unwrap_or(contract.start);
    let cliff_amount = contract.cliff_amount.unwrap_or(0);
    //todo: floats are imprecise, use integer division (requires witty solutions, adds complexity)
    //todo need checked math bcs of under/overflow
    let num_periods = (contract.end - cliff) as f64 / contract.period as f64;
    let period_amount = (contract.deposited_amount - cliff_amount) as f64 / num_periods;
    let periods_passed: u64 = (now - cliff) / contract.period;
    return (periods_passed as f64 * period_amount) as u64 + cliff_amount - contract.withdrawn;
}

pub fn transfer_tokens<'a>(from: &AccountInfo<'a>, to: &AccountInfo<'a>, auth: &AccountInfo<'a>, token_program: &Program<'a, Token>, signer_seeds: &[&[&[u8]]], amount: u64) -> ProgramResult {
    let ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &from.key(),
        &to.key(),
        &auth.key(),
        &[],
        amount,
    )?;
    invoke_signed(
        &ix,
        &[
            //account order is important!
            from.to_account_info(),
            to.to_account_info(),
            auth.to_account_info(),
            token_program.to_account_info(),
        ],
        signer_seeds,
    )
}

pub fn is_valid_schedule(start: u64, end: u64, now: u64, cliff: Option<u64>) -> bool {
    //we'll add more to it with the vesting periods
    let cliff_condition = if cliff.is_none() { true } else { start <= cliff.unwrap() && cliff.unwrap() <= end };
    now < start && start < end && cliff_condition
}

//todo use this helper
// pub fn get_signer<'a, 'b, 'c>(pda: &Account<'static, VestingContract>) -> &'a[&'b[&'c[u8]]; 1] {
//     let seeds = &[
//         pda.to_account_info().key.as_ref(),
//         &[pda.nonce],
//     ];
//     &[&seeds[..]]
// }
