use anchor_lang::prelude::*;
use anchor_spl::{
    token,
    associated_token::AssociatedToken,
    token::{Mint, MintTo, Token, TokenAccount, Transfer, Burn},
};
use std::{cmp::max};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod anchor_stake {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> ProgramResult {
        // initalize all the accounts for a new tokenX pool 
        Ok(())
    }
    
    pub fn new_staker(ctx: Context<NewStaker>) -> ProgramResult {    
        // initalize all the accounts for a new staker on tokenX 
        Ok(())
    }

    pub fn add(ctx: Context<Operation>, deposit_amount: u64) -> ProgramResult {    

        let reciept = &mut ctx.accounts.reciept;
        // record new staked add 
        if reciept.is_valid == 0 {
            reciept.is_valid = 1;
            reciept.created_ts = ctx.accounts.clock.unix_timestamp;
            reciept.amount_deposited = deposit_amount;
        } else { 
            // cant stake twice 
            return Err(ErrorCode::AccountAlreadyStakedError.into());
        }

        // transfer X token from sender -> PDA vault 
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Transfer {
                from: ctx.accounts.sender_token_x.to_account_info(), 
                to: ctx.accounts.vault_x.to_account_info(),
                authority: ctx.accounts.sender.to_account_info(), 
            }
        );
        token::transfer(transfer_ctx, deposit_amount)?;

        // transfer synthetic X to sender 
        let mint_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            MintTo {
                to: ctx.accounts.sender_token_synth_x.to_account_info(),
                mint: ctx.accounts.synthetic_x.to_account_info(),
                authority: ctx.accounts.synthetic_x.to_account_info(),
            }
        );
        let bump = *ctx.bumps.get("synthetic_x").unwrap();
        let tokenx_key = ctx.accounts.token_x.key();
        let pda_sign = &[
            b"synthetic",
            tokenx_key.as_ref(),
            &[bump],
        ];
        token::mint_to(
            mint_ctx.with_signer(&[pda_sign]), 
            deposit_amount
        )?;

        Ok(())
    }   

    pub fn remove(ctx: Context<Operation>) -> ProgramResult {

        // compute bonus for staking 
        let reciept = &mut ctx.accounts.reciept;
        if reciept.is_valid == 0 { // must have staked in order to remove
            return Err(ProgramError::InvalidAccountData)
        }
        let deposited_amount = reciept.amount_deposited;
        let start_time = reciept.created_ts; 
        let curr_time = ctx.accounts.clock.unix_timestamp; 
        
        // ~1 reward per second (note: unix time isnt always perfect)
        let diff_time = curr_time - start_time;
        // compute burn amount after rewards for staking 
        let burn_amount = max(0, deposited_amount - diff_time as u64);

        // reset reciept validity 
        reciept.is_valid = 0; 

        // remove SynthX from sender 
        if burn_amount > 0 {
            let burn_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(), 
                Burn {
                    mint: ctx.accounts.synthetic_x.to_account_info(),
                    to: ctx.accounts.sender_token_synth_x.to_account_info(),
                    authority: ctx.accounts.sender.to_account_info()
                }
            );
            token::burn(burn_ctx, burn_amount)?;
        }

        // send back the deposited tokens 
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Transfer {
                from: ctx.accounts.vault_x.to_account_info(), 
                to: ctx.accounts.sender_token_x.to_account_info(),
                authority: ctx.accounts.vault_x.to_account_info(), 
            }
        );
        let bump = *ctx.bumps.get("vault_x").unwrap();
        let tokenx_key = ctx.accounts.token_x.key();
        let pda_sign = &[
            b"vault",
            tokenx_key.as_ref(),
            &[bump],
        ];

        token::transfer(
            transfer_ctx.with_signer(&[pda_sign]), 
            deposited_amount
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    pub token_x: Account<'info, Mint>,
    // synthetic version of token X
    #[account(
        init, 
        payer=payer,
        seeds=[b"synthetic", token_x.key().as_ref()], 
        bump, 
        mint::decimals = token_x.decimals,
        mint::authority = synthetic_x
    )] 
    pub synthetic_x: Account<'info, Mint>, 
    // account to hold token X
    #[account(
        init, 
        payer=payer, 
        seeds=[b"vault", token_x.key().as_ref()], 
        bump,
        token::mint = token_x,
        token::authority = vault_x
    )]
    pub vault_x: Account<'info, TokenAccount>, 
    pub payer: Signer<'info>,
    // accounts required to init a new mint
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct NewStaker<'info> {
    pub token_x: Account<'info, Mint>,
    #[account(init, payer=sender, seeds=[b"reciept", token_x.key().as_ref(), sender.key().as_ref()], bump)] 
    pub reciept: Account<'info, Receipt>,
    pub sender: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Operation<'info> {
    pub token_x: Account<'info, Mint>,
    #[account(mut, seeds=[b"synthetic", token_x.key().as_ref()], bump)] 
    pub synthetic_x: Account<'info, Mint>, // mint of synthetic token X
    #[account(mut, seeds=[b"vault", token_x.key().as_ref()], bump)] 
    pub vault_x: Account<'info, TokenAccount>, // mint to hold token X
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(mut)]
    pub sender_token_x: Account<'info, TokenAccount>,
    #[account(mut)]
    pub sender_token_synth_x: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    #[account(mut, seeds=[b"reciept", token_x.key().as_ref(), sender.key().as_ref()], bump)] 
    pub reciept: Account<'info, Receipt>,
}

#[account]
#[derive(Default)] // will be init to zeros 
pub struct Receipt {
    pub is_valid: u8,
    pub created_ts: i64,
    pub amount_deposited: u64,
}


#[error]
pub enum ErrorCode {
    #[msg("Account has already staked.")]
    AccountAlreadyStakedError,
}