use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

use streamflow_timelock::{
    associated_token::{cancel_token_stream, initialize_token_stream, withdraw_token_stream},
    state::{CancelAccounts, InitializeAccounts, StreamInstruction, WithdrawAccounts},
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod timelock {
    use super::*;

    pub fn create(
        ctx: Context<Create>,
        amount: u64,
        start_time: u64,
        end_time: u64,
        period: u64,
        cliff: u64,
        cliff_amount: u64,
    ) -> ProgramResult {
        let ix = StreamInstruction {
            start_time,
            end_time,
            amount,
            period,
            cliff,
            cliff_amount,
        };

        let acc = InitializeAccounts {
            sender_wallet: ctx.accounts.sender_wallet.to_account_info(),
            sender_tokens: ctx.accounts.sender_tokens.to_account_info(),
            recipient_wallet: ctx.accounts.recipient_wallet.to_account_info(),
            recipient_tokens: ctx.accounts.recipient_tokens.to_account_info(),
            metadata_account: ctx.accounts.metadata_account.to_account_info(),
            escrow_account: ctx.accounts.escrow_account.to_account_info(),
            mint_account: ctx.accounts.mint_account.to_account_info(),
            rent_account: ctx.accounts.rent_account.to_account_info(),
            timelock_program_account: ctx.accounts.timelock_program_account.to_account_info(),
            token_program_account: ctx.accounts.token_program_account.to_account_info(),
            associated_token_program_account: ctx
                .accounts
                .associated_token_program_account
                .to_account_info(),
            system_program_account: ctx.accounts.system_program_account.to_account_info(),
        };

        initialize_token_stream(ctx.program_id, acc, ix)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> ProgramResult {
        let acc = WithdrawAccounts {
            sender_wallet: ctx.accounts.sender_wallet.to_account_info(),
            sender_tokens: ctx.accounts.sender_tokens.to_account_info(),
            recipient_wallet: ctx.accounts.recipient_wallet.to_account_info(),
            recipient_tokens: ctx.accounts.recipient_tokens.to_account_info(),
            metadata_account: ctx.accounts.metadata_account.to_account_info(),
            escrow_account: ctx.accounts.metadata_account.to_account_info(),
            mint_account: ctx.accounts.mint_account.to_account_info(),
            timelock_program_account: ctx.accounts.timelock_program_account.to_account_info(),
            token_program_account: ctx.accounts.token_program_account.to_account_info(),
            system_program_account: ctx.accounts.system_program_account.to_account_info(),
        };

        withdraw_token_stream(ctx.program_id, acc, amount)
    }

    pub fn cancel(ctx: Context<Cancel>) -> ProgramResult {
        let acc = CancelAccounts {
            sender_wallet: ctx.accounts.sender_wallet.to_account_info(),
            sender_tokens: ctx.accounts.sender_tokens.to_account_info(),
            recipient_wallet: ctx.accounts.recipient_wallet.to_account_info(),
            recipient_tokens: ctx.accounts.recipient_tokens.to_account_info(),
            metadata_account: ctx.accounts.metadata_account.to_account_info(),
            escrow_account: ctx.accounts.metadata_account.to_account_info(),
            mint_account: ctx.accounts.mint_account.to_account_info(),
            timelock_program_account: ctx.accounts.timelock_program_account.to_account_info(),
            token_program_account: ctx.accounts.token_program_account.to_account_info(),
            system_program_account: ctx.accounts.system_program_account.to_account_info(),
        };

        cancel_token_stream(ctx.program_id, acc)
    }
}

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(mut)]
    pub sender_wallet: AccountInfo<'info>,
    #[account(mut)]
    pub sender_tokens: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_wallet: AccountInfo<'info>,
    #[account(mut)]
    pub recipient_tokens: Account<'info, TokenAccount>,
    #[account(mut)]
    pub metadata_account: AccountInfo<'info>,
    #[account(mut)]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account()]
    pub mint_account: ProgramAccount<'info, Mint>,
    #[account()]
    pub rent_account: Sysvar<'info, Rent>,
    #[account()]
    pub timelock_program_account: AccountInfo<'info>,
    #[account()]
    pub token_program_account: Program<'info, Token>,
    #[account()]
    pub associated_token_program_account: Program<'info, AssociatedToken>,
    #[account()]
    pub system_program_account: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub sender_wallet: AccountInfo<'info>,
    #[account(mut)]
    pub sender_tokens: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_wallet: AccountInfo<'info>,
    #[account(mut)]
    pub recipient_tokens: Account<'info, TokenAccount>,
    #[account(mut)]
    pub metadata_account: AccountInfo<'info>,
    #[account(mut)]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account()]
    pub mint_account: ProgramAccount<'info, Mint>,
    #[account()]
    pub timelock_program_account: AccountInfo<'info>,
    #[account()]
    pub token_program_account: Program<'info, Token>,
    #[account()]
    pub system_program_account: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub sender_wallet: AccountInfo<'info>,
    #[account(mut)]
    pub sender_tokens: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_wallet: AccountInfo<'info>,
    #[account(mut)]
    pub recipient_tokens: Account<'info, TokenAccount>,
    #[account(mut)]
    pub metadata_account: AccountInfo<'info>,
    #[account(mut)]
    pub escrow_account: Account<'info, TokenAccount>,
    #[account()]
    pub mint_account: ProgramAccount<'info, Mint>,
    #[account()]
    pub timelock_program_account: AccountInfo<'info>,
    #[account()]
    pub token_program_account: Program<'info, Token>,
    #[account()]
    pub system_program_account: Program<'info, System>,
}
