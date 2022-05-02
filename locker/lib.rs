use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;
declare_id!("..");

const LOCK_PDA_SEED: &[u8] = b"locker";

#[program]
pub mod liquidity_lock {
    use super::*;
    pub fn create_lock(ctx: Context<CreateLock>,bump: u8,seed:u8,to_lock_amount: u64,lock_duration:i64) -> Result<()> {
        let clock = Clock::get()?;

        ctx.accounts.locker_account.lp_token_address = ctx.accounts.mint.key();
        ctx.accounts.locker_account.unlock_date = clock.unix_timestamp + lock_duration;
        ctx.accounts.locker_account.lp_token_locked_quantity = to_lock_amount;
        ctx.accounts.locker_account.seed = seed;
        ctx.accounts.locker_account.temp_token_account = ctx.accounts.temp_token_account.key();
        ctx.accounts.locker_account.initialized = true;
        ctx.accounts.locker_account.owner = ctx.accounts.initializer.key();

        let (locker_authority, _locker_authority_bump) =
        Pubkey::find_program_address(&[LOCK_PDA_SEED], ctx.program_id);

    token::set_authority(
            ctx.accounts.into_set_authority_context(),
            AuthorityType::AccountOwner,
            Some(locker_authority),
    )?;

    token::transfer(
            ctx.accounts.into_transfer_to_temp_token_context(),
            to_lock_amount,
    )?;



        Ok(())
    }
}
pub fn extend_lock(ctx: Context<ExtendLock>,bump: u8,seed:u8,extend_duration:i64) -> Result<()> {
    let clock = Clock::get()?;
    if extend_duration < 0 {


        return Err(error!(ErrorCode::InvalidExtendDuration));
    }

    ctx.accounts.locker_account.unlock_date += extend_duration;
 



    Ok(())
}

pub fn withdraw_unlocked(ctx: Context<WithdrawUnlocked>,bump: u8,seed:u8) -> Result<()> {
    let clock = Clock::get()?;

    if   clock.unix_timestamp > ctx.accounts.locker_account.unlock_date{
        return Err(error!(ErrorCode::CantWithdraw));


    }

    let (locker_authority, _locker_authority_bump) =
    Pubkey::find_program_address(&[LOCK_PDA_SEED], ctx.program_id);

    let authority_seeds = &[&LOCK_PDA_SEED[..], &[_locker_authority_bump]];


    token::transfer(
        ctx.accounts
            .into_transfer_to_taker_context()
            .with_signer(&[&authority_seeds[..]]),
            ctx.accounts.locker_account.lp_token_locked_quantity,
    )?;




    Ok(())
}


#[derive(Accounts)]
#[instruction(bump: u8,seed:u8,to_lock_amount: u64)]
pub struct CreateLock<'info> {

    #[account(mut, signer)]
    pub initializer: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = initializer,
        token::mint = mint,
        token::authority = initializer,
    )]
    pub temp_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        seeds = [&[seed]],
        bump,
        payer = initializer,        
        space = 44+ 1 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8+ 44 + 44

    )]
    pub locker_account: Account<'info,LockerAccount>,
    #[account(
        mut,
        constraint = initializer_deposit_token_account.amount >= to_lock_amount,
        constraint = initializer_deposit_token_account.mint == *mint.to_account_info().key,
    )]
    pub initializer_deposit_token_account: Account<'info, TokenAccount>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,
    pub token_program: AccountInfo<'info>,





}


#[derive(Accounts)]
#[instruction(bump: u8,seed:u8)]
pub struct ExtendLock<'info> {

    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
    #[account(

        constraint= temp_token_account.mint == locker_account.lp_token_address,
        
  
    )]
    pub temp_token_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [&[seed]],
        bump,
        has_one = temp_token_account,
        has_one = owner




    )]
    pub locker_account: Account<'info,LockerAccount>,







}
#[derive(Accounts)]
#[instruction(bump: u8,seed:u8)]
pub struct WithdrawUnlocked<'info> {

    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
    #[account(

        constraint= temp_token_account.mint == locker_account.lp_token_address,
        
  
    )]

    #[account(
        mut,
        has_one = owner,
        has_one = mint

    )]
    pub owner_token_account: Account<'info, TokenAccount>,
    pub temp_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [&[seed]],
        bump,
        has_one = temp_token_account,
        has_one = owner,
        close = owner





    )]
    pub locker_account: Account<'info,LockerAccount>,

    pub locker_authority: AccountInfo<'info>,
    
    pub token_program: AccountInfo<'info>







}
#[account]
pub struct LockerAccount {

    lp_token_address: Pubkey,

    unlock_date: i64,

    lp_token_locked_quantity: u64,

    seed: u8,

    temp_token_account: Pubkey,

    initialized: bool,

    owner: Pubkey,
    
    
}
impl<'info> CreateLock<'info> {
    fn into_transfer_to_temp_token_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .initializer_deposit_token_account
                .to_account_info()
                .clone(),
            to: self.temp_token_account.to_account_info().clone(),
            authority: self.initializer.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.temp_token_account.to_account_info().clone(),
            current_authority: self.initializer.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

impl<'info> WithdrawUnlocked<'info> {
   

    fn into_transfer_to_taker_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        
        let cpi_accounts = Transfer {
            from:  self.temp_token_account.to_account_info().clone(),
            to: self.owner_token_account.to_account_info().clone(),
            authority: self.locker_authority.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

  
}

#[error_code]
pub enum ErrorCode {
    #[msg("Extend Duration Can't be Lower than 0")]
    InvalidExtendDuration,
    #[msg("Not able to withdraw yet")]
    CantWithdraw,
    



}

