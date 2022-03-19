use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;
declare_id!("FCQH7SonwKkekepgjUd6w6uatQSVxkQMmMtgxgxaceCy");


const CAMPAIGN_PDA_SEED: &[u8] = b"campaign";
const CONFIG_PDA_SEED: &[u8] = b"config";


#[program]
pub mod unilock {
    use super::*;
    pub fn initialize(ctx: Context<InitializeCampaign>,
        bump: u8,
        hard_cap:u64,
        soft_cap:u64,
        presale_buy_rate:u64,
        max_per_wallet: u64,
        min_per_wallet: u64,
        start_date_timestamp: i64,
        end_date_timestamp: i64,
    ) -> Result<()> {
        

        let campaign_account = &mut ctx.accounts.campaign_account;
        campaign_account.token_address = *ctx.accounts.mint.to_account_info().key;
        campaign_account.hard_cap = hard_cap;
        campaign_account.soft_cap = soft_cap;
        campaign_account.presale_buy_rate = presale_buy_rate;
        campaign_account.temp_token_account = *ctx.accounts.temp_token_account.to_account_info().key;
        campaign_account.initialized = true ;
        campaign_account.owner = *ctx.accounts.initializer.key;
        campaign_account.total_lamports_collected = 0;
        campaign_account.max_per_wallet = max_per_wallet;
        campaign_account.min_per_wallet = min_per_wallet;
        campaign_account.start_date_timestamp = start_date_timestamp;
        campaign_account.end_date_timestamp = end_date_timestamp;
        campaign_account.succeeded = false;




        let (campaign_authority, _campaign_authority_bump) =
            Pubkey::find_program_address(&[CAMPAIGN_PDA_SEED], ctx.program_id);

        token::set_authority(
                ctx.accounts.into_set_authority_context(),
                AuthorityType::AccountOwner,
                Some(campaign_authority),
        )?;
    
        token::transfer(
                ctx.accounts.into_transfer_to_temp_token_context(),
                (hard_cap * presale_buy_rate) / u64::pow(10, 9),
        )?;
        Ok(())
    }
    pub fn buy_token(ctx: Context<BuyToken>,
        bump: u8,
        campaign_bump: u8,
        contributed_lamports: u64) -> Result<()> {
        let clock = Clock::get()?;
        if !is_live(
            ctx.accounts.campaign_account.clone(),
            clock.unix_timestamp,
        ) || is_failed(
            ctx.accounts.campaign_account.clone(),
            clock.unix_timestamp,
        ) {

            msg!("Current timestamp {}",clock.unix_timestamp);
            msg!("Campaign is not live");
            return Err(error!(ErrorCode::CampaignOFF));
        }
        if ctx.accounts.campaign_account.hard_cap < ctx.accounts.campaign_account.total_lamports_collected + contributed_lamports {

            msg!("Can't contribute more than hardcap");
            return Err(error!(ErrorCode::HardCapThreshhold));
        }

        if ctx.accounts.campaign_account.max_per_wallet < ctx.accounts.buyer_account.contributed_lamports + contributed_lamports {
            msg!("Can't buy more than you are allowed to");
            return Err(error!(ErrorCode::MaxPerWallet));
        }
        if (ctx.accounts.campaign_account.min_per_wallet > contributed_lamports) && ( ctx.accounts.campaign_account.min_per_wallet < ctx.accounts.campaign_account.hard_cap - ctx.accounts.campaign_account.total_lamports_collected) {
            msg!("Can't buy less than you are required to");
            return Err(error!(ErrorCode::MinPerWallet));
        }

        if ctx.accounts.buyer_account.initialized == false {

            ctx.accounts.buyer_account.initialized = true;
            ctx.accounts.buyer_account.initializer = *ctx.accounts.buyer.key;
            ctx.accounts.buyer_account.claimed = false;
            ctx.accounts.buyer_account.campaign_account = *ctx.accounts.campaign_account.to_account_info().key;
            ctx.accounts.buyer_account.contributed_lamports = 0;
            ctx.accounts.buyer_account.owner = *ctx.accounts.buyer.key;

        }
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.buyer.key(),
            &ctx.accounts.campaign_account.to_account_info().key(),
            contributed_lamports,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.campaign_account.to_account_info(),
            ],
        );
        ctx.accounts.buyer_account.contributed_lamports += contributed_lamports;
        ctx.accounts.campaign_account.total_lamports_collected += contributed_lamports;

        
        Ok(())
    }
    pub fn withdraw_tokens(ctx: Context<WithdrawTokens>, campaign_bump: u8,
        buyer_bump: u8) -> Result<()> {
        let clock = Clock::get()?;

        if is_live(
            ctx.accounts.campaign_account.clone(),
            clock.unix_timestamp,
        ) || is_failed(
            ctx.accounts.campaign_account.clone(),
            clock.unix_timestamp,
        ) {

            return Err(error!(ErrorCode::CanWithdrawTokens));
        }
        let (campaign_authority, _campaign_authority_bump) =
            Pubkey::find_program_address(&[CAMPAIGN_PDA_SEED], ctx.program_id);

        if *ctx.accounts.campaign_authority.key != campaign_authority {
            return Err(error!(ErrorCode::InvalidAuthority));

        }    

        let authority_seeds = &[&CAMPAIGN_PDA_SEED[..], &[_campaign_authority_bump]];
        

        token::transfer(
            ctx.accounts
                .into_transfer_to_taker_context()
                .with_signer(&[&authority_seeds[..]]),
            ctx.accounts.buyer_account.contributed_lamports * ctx.accounts.campaign_account.presale_buy_rate / u64::pow(10, 9),
        )?;
 
        
        Ok(())
    }
    pub fn withdraw_funds(ctx: Context<WithdrawFunds>,campaign_bump: u8,
        buyer_bump: u8,    config_bump:u8
    ) -> Result<()> {
        let clock = Clock::get()?;

        if !is_failed(
            ctx.accounts.campaign_account.clone(),
            clock.unix_timestamp,
        ) {
            msg!("Can't withdraw funds");
            return Err(error!(ErrorCode::CantWithdrawFunds));
        }
      
       


        // ctx.accounts.campaign_account
        let  to_distribute_amount= ctx.accounts.buyer_account.contributed_lamports * ctx.accounts.config_account.fee / 1000 ;
        let to_fees_amount = ctx.accounts.buyer_account.contributed_lamports - to_distribute_amount ;

        **ctx.accounts.campaign_account.to_account_info().try_borrow_mut_lamports()? -=  ctx.accounts.buyer_account.contributed_lamports;
        **ctx.accounts.owner.to_account_info().try_borrow_mut_lamports()? += to_distribute_amount;
        **ctx.accounts.to_address.to_account_info().try_borrow_mut_lamports()? += to_fees_amount;

    
        
        Ok(())
    }
    pub fn distribute_funds(ctx: Context<DistributeFunds>, campaign_bump: u8,config_bump:u8

        ) -> Result<()> {
        let clock = Clock::get()?;

        if is_live(
            ctx.accounts.campaign_account.clone(),
            clock.unix_timestamp,
        ) || is_failed(
            ctx.accounts.campaign_account.clone(),
            clock.unix_timestamp,
        ) {

            msg!("Current timestamp {}",clock.unix_timestamp);
            msg!("Campaign is not live");
            return Err(error!(ErrorCode::CantWithdrawFunds));
        }
        if ctx.accounts.campaign_account.succeeded == true {
            msg!("Campaign already succeeded");
            return Err(error!(ErrorCode::AlreadySucceeded));


            
        }
        ctx.accounts.campaign_account.succeeded = true;

        let  to_distribute_amount= ctx.accounts.campaign_account.total_lamports_collected * ctx.accounts.config_account.fee / 1000 ;
        let to_fees_amount = ctx.accounts.campaign_account.total_lamports_collected  - to_distribute_amount ;

        **ctx.accounts.campaign_account.to_account_info().try_borrow_mut_lamports()? -=  ctx.accounts.campaign_account.total_lamports_collected;
        **ctx.accounts.owner.to_account_info().try_borrow_mut_lamports()? += to_distribute_amount;
        **ctx.accounts.to_address.to_account_info().try_borrow_mut_lamports()? += to_fees_amount;




        
        Ok(())
    }
    pub fn init_config_account(ctx: Context<InitConfigAccount>, bump: u8,fee : u64
    ) -> Result<()> {

        if ctx.accounts.config_account.initialized {
            return Err(error!(ErrorCode::CongfigInitialized));
 
        }
        ctx.accounts.config_account.initialized = true;
        ctx.accounts.config_account.to_address = *ctx.accounts.to_address.key;
        ctx.accounts.config_account.fee = fee;
        ctx.accounts.config_account.owner =  *ctx.accounts.owner.key;
    





    
    Ok(())
  }
  pub fn edit_config_account(ctx: Context<EditConfigAccount>, bump: u8,fee : u64
  ) -> Result<()> {

      if ctx.accounts.config_account.initialized {
          return Err(error!(ErrorCode::CongfigInitialized));

      }
      ctx.accounts.config_account.initialized = true;
      ctx.accounts.config_account.to_address = *ctx.accounts.to_address.key;
      ctx.accounts.config_account.fee = fee;
      ctx.accounts.config_account.owner =  *ctx.accounts.owner.key;
  





  
  Ok(())
}

   
}
fn is_live(campaign: Account<'_, CampaignAccount, >, current_time: i64) -> bool {
    if campaign.total_lamports_collected >= campaign.hard_cap {
        return false;
    }else if (current_time > campaign.end_date_timestamp)
        || (current_time < campaign.start_date_timestamp)
    {
        return false;
    }else {

        return true;
    }
}

fn is_failed(campaign: Account<'_, CampaignAccount, >, current_time: i64) -> bool {
    if (campaign.total_lamports_collected < campaign.soft_cap)
        && (current_time > campaign.end_date_timestamp)
    {
        return true;
    }else{
        return false;

    }
}


#[derive(Accounts)]
#[instruction(bump: u8,hard_cap: u64,presale_buy_rate:u64)]
pub struct InitializeCampaign<'info> {

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
        seeds = [&mint.to_account_info().key.as_ref().to_owned()[0..9]],
        bump,
        payer = initializer,        
        space = 44+ 1 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8+ 44 + 44

    )]
    pub campaign_account: Account<'info,CampaignAccount>,
    #[account(
        mut,
        constraint = initializer_deposit_token_account.amount >= (presale_buy_rate*hard_cap) / u64::pow(10, 9),
        constraint = initializer_deposit_token_account.mint == *mint.to_account_info().key,
    )]
    pub initializer_deposit_token_account: Account<'info, TokenAccount>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,
    pub token_program: AccountInfo<'info>,





}
#[account]
pub struct CampaignAccount {

    token_address: Pubkey,

    hard_cap: u64,

    soft_cap: u64,

    max_per_wallet: u64,

    min_per_wallet: u64,

    presale_buy_rate: u64,

    start_date_timestamp: i64,

    end_date_timestamp: i64,

    total_lamports_collected: u64,

    temp_token_account: Pubkey,

    initialized: bool,

    succeeded: bool,

    owner: Pubkey,
    
    
}

#[derive(Accounts)]
#[instruction(
    bump: u8,
    campaign_bump: u8,
    contributed_lamports: u64)]
    pub struct BuyToken<'info>{
   
    #[account(mut, signer)]
    pub buyer: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,

    #[account(
       mut,
       seeds = [&mint.to_account_info().key.as_ref().to_owned()[0..9]],
       bump = campaign_bump,
       constraint = campaign_account.token_address == *mint.to_account_info().key


    )]
    pub campaign_account: Account<'info,CampaignAccount>,
    #[account(
        init_if_needed,
        payer = buyer,
        seeds = [&campaign_account.to_account_info().key.as_ref().to_owned()[0..4],&buyer.to_account_info().key.as_ref().to_owned()[0..5]],
        bump,
        space = 44 + 8 + 2 + 44 + 2 + 44,
        



     )]
    pub buyer_account: Account<'info,BuyerAccount>,


    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,




}

#[derive(Accounts)]
#[instruction(
    campaign_bump: u8,
    buyer_bump: u8,
    config_bump:u8

)]
pub struct WithdrawFunds<'info>{
   
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,


    #[account(
       mut,
       seeds = [&mint.to_account_info().key.as_ref().to_owned()[0..9]],
       bump = campaign_bump,
       constraint = campaign_account.token_address == *mint.to_account_info().key


    )]
    pub campaign_account: Account<'info,CampaignAccount>,
    #[account(
       mut,
       has_one = campaign_account,
       has_one = owner,
       close = owner,
       seeds = [&campaign_account.to_account_info().key.as_ref().to_owned()[0..4],&owner.to_account_info().key.as_ref().to_owned()[0..5]],
       bump = buyer_bump,



     )]
    pub buyer_account: Account<'info,BuyerAccount>,
    #[account(
        mut,

 
 
      )]
    pub to_address :AccountInfo<'info>,
    #[account
    (
    
        seeds = [CONFIG_PDA_SEED],
        bump = config_bump,
        has_one = to_address


    
    )]

    pub config_account: Account<'info,ConfigAccount>,





}
#[derive(Accounts)]
#[instruction(
    campaign_bump: u8,
    config_bump:u8
)]
pub struct DistributeFunds<'info>{
   
    #[account(mut, signer)]
    pub signer: AccountInfo<'info>,

    pub mint: Account<'info, Mint>,


 #[account(
       mut,
    )]
    pub owner: AccountInfo<'info>,



    #[account(
       mut,
       seeds = [&mint.to_account_info().key.as_ref().to_owned()[0..9]],
       bump,
       has_one = owner,
       constraint = campaign_account.token_address == *mint.to_account_info().key

    )]
    pub campaign_account: Account<'info,CampaignAccount>,
    pub to_address :AccountInfo<'info>,
    #[account
    (
    
        seeds = [CONFIG_PDA_SEED],
        bump = config_bump,
        has_one = to_address


    
    )]

    pub config_account: Account<'info,ConfigAccount>,





}

#[derive(Accounts)]
#[instruction(
    bump: u8,
)]
pub struct InitConfigAccount<'info> {

    to_address:  AccountInfo<'info>,

    #[account(mut,signer)]

    owner: AccountInfo<'info>,

    #[account
    (
        init,
        payer = owner,
        space = 44 + 8 + 44 +2,
        seeds = [CONFIG_PDA_SEED],
        bump


    
    )]

    config_account: Account<'info,ConfigAccount>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,


    
}

#[derive(Accounts)]
#[instruction(
    bump: u8,
)]
pub struct EditConfigAccount<'info> {

    to_address:  AccountInfo<'info>,

    #[account(mut,signer)]

    owner: AccountInfo<'info>,

    #[account
    (

        seeds = [CONFIG_PDA_SEED],
        bump,
        has_one = owner


    
    )]

    config_account: Account<'info,ConfigAccount>,



    
}


#[account]
pub struct ConfigAccount {

    to_address: Pubkey,

    fee: u64,

    owner: Pubkey,

    initialized: bool,

    
    
}



#[account]
pub struct BuyerAccount {
    
    campaign_account: Pubkey,
    contributed_lamports: u64,
    initialized: bool,
    initializer: Pubkey,
    owner : Pubkey,
    claimed: bool,
    
    
}
#[derive(Accounts)]
#[instruction(
    campaign_bump: u8,
    buyer_bump: u8

)]
pub struct WithdrawTokens<'info>{
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        has_one = owner,
        has_one = mint

    )]
    pub owner_token_account: Account<'info, TokenAccount>,



    #[account(
       mut,
       has_one = temp_token_account,
       
       seeds = [&mint.to_account_info().key.as_ref().to_owned()[0..9]],
       bump = campaign_bump,
       constraint = campaign_account.token_address == *mint.to_account_info().key


    )]
    pub campaign_account: Account<'info,CampaignAccount>,
    #[account(
       mut,
       has_one = campaign_account,
       has_one = owner,
       close = owner,
       seeds = [&campaign_account.to_account_info().key.as_ref().to_owned()[0..4],&owner.to_account_info().key.as_ref().to_owned()[0..5]],
       bump = buyer_bump

     )]
    pub buyer_account: Account<'info,BuyerAccount>,

    #[account(mut)]
    pub temp_token_account: Account<'info, TokenAccount>,
    pub token_program: AccountInfo<'info>,
    pub campaign_authority: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>



}


impl<'info> InitializeCampaign<'info> {
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
impl<'info> WithdrawTokens<'info> {
   

    fn into_transfer_to_taker_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        
        let cpi_accounts = Transfer {
            from:  self.temp_token_account.to_account_info().clone(),
            to: self.owner_token_account.to_account_info().clone(),
            authority: self.campaign_authority.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

  
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid end date")]
    InvalidEndDate,
    #[msg("Invalid hard cap")]
    InvalidHardcap,
    #[msg("Campaign is not live")]
    CampaignOFF,
    #[msg("Can't contribute more than hardcap")]
    HardCapThreshhold,
    #[msg("Can't buy more than you are allowed to")]
    MaxPerWallet,
    #[msg("Can't buy less than you are required to")]
    MinPerWallet,
    #[msg("Can't withdraw funds")]
    CantWithdrawFunds,
    #[msg("Can't withdraw tokens")]
    CanWithdrawTokens,
    #[msg("Invalid Authority")]
    InvalidAuthority,
    #[msg("Campaign already succeeded")]
    AlreadySucceeded,
    #[msg("Config Already initialized")]
    CongfigInitialized



}
