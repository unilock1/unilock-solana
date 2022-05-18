use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;
declare_id!("...");

use serum_dex::instruction::initialize_market;
use amm_anchor;

const CAMPAIGN_PDA_SEED: &[u8] = b"campaign";
const CONFIG_PDA_SEED: &[u8] = b"config";


#[program]

pub mod unilock {
    use super::*;
    #[inline(never)]
    pub fn initialize(ctx: Context<InitializeCampaign>,
        bump: u8,
        hard_cap:u64,
        soft_cap:u64,
        presale_buy_rate:u64,
        max_per_wallet: u64,
        min_per_wallet: u64,
        raydium_percentage : u64,
        listing_price : u64,
        start_date_timestamp: i64,
        end_date_timestamp: i64,
    ) -> Result<()> {
        
        let campaign_account = &mut ctx.accounts.campaign_account;
        campaign_account.token_address = *ctx.accounts.mint.to_account_info().key;
        campaign_account.pc_token_address = *ctx.accounts.pc_mint.to_account_info().key;
        campaign_account.hard_cap = hard_cap;
        campaign_account.soft_cap = soft_cap;
        campaign_account.presale_buy_rate = presale_buy_rate;
        campaign_account.temp_token_account = *ctx.accounts.temp_token_account.to_account_info().key;
        campaign_account.pc_temp_token_account = *ctx.accounts.pc_temp_token_account.to_account_info().key;
        campaign_account.initialized = true ;
        campaign_account.owner = *ctx.accounts.initializer.key;
        campaign_account.total_lamports_collected = 0;
        campaign_account.max_per_wallet = max_per_wallet;
        campaign_account.min_per_wallet = min_per_wallet;
        campaign_account.start_date_timestamp = start_date_timestamp;
        campaign_account.end_date_timestamp = end_date_timestamp;
        campaign_account.raydium_percentage = raydium_percentage;
        campaign_account.listing_price = listing_price;
        campaign_account.succeeded = false;

        let coin_decimals =  ctx.accounts.mint.decimals;

        let token_to_ray =( (((hard_cap * raydium_percentage as u64 ) / 1000) * listing_price as u64 ) / u64::pow(10, 9) ) * u64::pow(10, coin_decimals as u32) ;
        let to_transfer_tokens =(presale_buy_rate*hard_cap) / u64::pow(10, 9) + token_to_ray;

        if ( ctx.accounts.initializer_deposit_token_account.amount <  to_transfer_tokens ){

            return Err(error!(ErrorCode::InvalidTokenAmount));

        }


        let (campaign_authority, _campaign_authority_bump) =
            Pubkey::find_program_address(&[CAMPAIGN_PDA_SEED], ctx.program_id);

        token::set_authority(
                ctx.accounts.into_set_authority_context(),
                AuthorityType::AccountOwner,
                Some(campaign_authority),
        )?;
        token::set_authority(
            ctx.accounts.into_set_pc_authority_context(),
            AuthorityType::AccountOwner,
            Some(campaign_authority),
        )?;
    
        token::transfer(
                ctx.accounts.into_transfer_to_temp_token_context(),
                to_transfer_tokens,
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
  

        let cpi_sol_accounts = Transfer {
            from: ctx.accounts.temp_wsol.to_account_info().clone(),
            to: ctx.accounts.pc_temp_token_account.to_account_info().clone(),
            authority: ctx.accounts.buyer.to_account_info().clone(),


        };
       
        let cpi_program_sol = ctx.accounts.token_program.clone();

        let cpi_sol_ctx = CpiContext::new(cpi_program_sol, cpi_sol_accounts);

        token::transfer(cpi_sol_ctx,contributed_lamports )?;
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
        msg!("authority seeds");


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
      
       


        let  to_distribute_amount= ctx.accounts.buyer_account.contributed_lamports * ctx.accounts.config_account.fee / 1000 ;
        let to_fees_amount = ctx.accounts.buyer_account.contributed_lamports - to_distribute_amount ;

        let (campaign_authority, _campaign_authority_bump) =
        Pubkey::find_program_address(&[CAMPAIGN_PDA_SEED], ctx.program_id);
        let authority_seeds = &[&CAMPAIGN_PDA_SEED[..], &[_campaign_authority_bump]];

        let cpi_sol_accounts = Transfer {
            from: ctx.accounts.pc_temp_token_account.to_account_info().clone(),
            to: ctx.accounts.owner_temp_token_account.to_account_info().clone(),
            authority: ctx.accounts.campaign_authority.clone(),


        };

    
        let cpi_program_sol = ctx.accounts.token_program.clone();
        let cpi_program_fees_sol = ctx.accounts.token_program.clone();

        let cpi_sol_ctx = CpiContext::new(cpi_program_sol, cpi_sol_accounts);

        token::transfer(cpi_sol_ctx.with_signer(&[authority_seeds]),to_distribute_amount)?;


    
        
        Ok(())
    }
   
    pub fn initialize_market(ctx: Context<InitializeMarket>,        campaign_bump : u8,    config_bump:u8
        ,
        baseLotSize: u64,
        quoteLotSize: u64,
        feeRateBp :u64,
        vaultSignerNonce:u64,
        quoteDustThreshold:u64,
            
        ) -> Result<()> {
           
    
            let ix = serum_dex::instruction::initialize_market(
                &ctx.accounts.market.key(),
                &ctx.accounts.serum_program.key(), 
                &ctx.accounts.baseMint.to_account_info().key,
                &ctx.accounts.quoteMint.to_account_info().key,
                &ctx.accounts.baseVault.key(),
                &ctx.accounts.quoteVault.key(),
                None,
                None,
                None,
                &ctx.accounts.bids.to_account_info().key, 
                &ctx.accounts.asks.to_account_info().key,
                &ctx.accounts.requestQueue.key(), 
                &ctx.accounts.eventQueue.key(),
                baseLotSize,
                quoteLotSize,
                vaultSignerNonce,
                quoteDustThreshold 
            ).unwrap();
    
            let (campaign_authority, _campaign_authority_bump) =
            Pubkey::find_program_address(&[CAMPAIGN_PDA_SEED], ctx.program_id);
            let authority_seeds = &[&CAMPAIGN_PDA_SEED[..], &[_campaign_authority_bump]];
    
    
            anchor_lang::solana_program::program::invoke_signed(
               &ix,
               &[
                ctx.accounts.campaign_authority.to_account_info(),
                ctx.accounts.market.to_account_info(),
                ctx.accounts.requestQueue.to_account_info(),
                ctx.accounts.eventQueue.to_account_info(),
                ctx.accounts.bids.to_account_info(),
                ctx.accounts.asks.to_account_info(),
                ctx.accounts.baseVault.to_account_info(),
                ctx.accounts.quoteVault.to_account_info(),
                ctx.accounts.baseMint.to_account_info(),
                ctx.accounts.quoteMint.to_account_info(),
                ctx.accounts.rent.to_account_info()
                ],&[&authority_seeds[..]]
    
    
            ).unwrap();
         
         
            Ok(())
        }
    pub fn addliquidity(ctx: Context<AddLiquidity>,campaign_bump: u8,    config_bump:u8
,        nonce: u8,open_time:u64,
            
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
        let (campaign_authority, _campaign_authority_bump) =
        Pubkey::find_program_address(&[CAMPAIGN_PDA_SEED], ctx.program_id);
        token::set_authority(
            ctx.accounts.into_set_authority_context(),
            AuthorityType::AccountOwner,
            Some(campaign_authority),
         )?;
            
          
        let ix = amm_anchor::instructions::initialize(

                &ctx.accounts.programId.key(),
                &ctx.accounts.amm_id.key(),
                &ctx.accounts.amm_authority.key(),
                &ctx.accounts.amm_open_orders.key(),
                &ctx.accounts.lp_mint_address.key(),
                &ctx.accounts.coin_mint_address.key(),
                &ctx.accounts.pc_mint_address.key(),
                &ctx.accounts.pool_coin_token_account.key(),
                &ctx.accounts.pool_pc_token_account.key(),
                &ctx.accounts.pool_withdraw_queue.key(),
                &ctx.accounts.pool_target_orders_account.key(),
                &ctx.accounts.pool_lp_token_account.key(),
                &ctx.accounts.pool_temp_lp_token_account.key(),
                &ctx.accounts.serum_program.key(),
                &ctx.accounts.serum_market.key(),
                &ctx.accounts.user_wallet.key(),
                nonce, 
                open_time

        ).unwrap();
        let (campaign_authority, _campaign_authority_bump) =
        Pubkey::find_program_address(&[CAMPAIGN_PDA_SEED], ctx.program_id);
        let authority_seeds = &[&CAMPAIGN_PDA_SEED[..], &[_campaign_authority_bump]];
        let coin_decimals =  ctx.accounts.coin_mint_address.decimals  ;
        
        let  to_distribute_amount= ctx.accounts.campaign_account.total_lamports_collected * ctx.accounts.configAccount.fee / 1000 ;
        let to_fees_amount = ctx.accounts.campaign_account.total_lamports_collected  - to_distribute_amount ;

        let sol_to_ray = (ctx.accounts.campaign_account.raydium_percentage as u64 * to_distribute_amount ) / 1000;
        let token_to_ray =  (sol_to_ray * ctx.accounts.campaign_account.listing_price as u64) *  u64::pow(10, coin_decimals as u32) /  u64::pow(10, 9) ;
        
        let cpi_sol_fees_accounts = Transfer {
            from: ctx.accounts.pc_temp_token_account.to_account_info().clone(),
            to: ctx.accounts.to_address.to_account_info().clone(),
            authority: ctx.accounts.campaign_authority.clone(),


        };
    
        let cpi_sol_accounts = Transfer {
            from: ctx.accounts.pc_temp_token_account.to_account_info().clone(),
            to: ctx.accounts.pool_pc_token_account.to_account_info().clone(),
            authority: ctx.accounts.campaign_authority.clone(),


        };
        let cpi_coin_accounts = Transfer {
            from: ctx.accounts.temp_token_account.to_account_info().clone(),
            to: ctx.accounts.pool_coin_token_account.to_account_info().clone(),
            authority: ctx.accounts.campaign_authority.clone(),
        };
        let cpi_program_sol = ctx.accounts.token_program.clone();
        let cpi_program_sol_fees = ctx.accounts.token_program.clone();

        let cpi_program_coin = ctx.accounts.token_program.clone();

        let cpi_sol_ctx = CpiContext::new(cpi_program_sol, cpi_sol_accounts);
        let cpi_sol_fees_ctx = CpiContext::new(cpi_program_sol_fees, cpi_sol_fees_accounts);

        let cpi_coin_ctx = CpiContext::new(cpi_program_coin, cpi_coin_accounts);

        token::transfer(cpi_sol_fees_ctx.with_signer(&[authority_seeds]),to_fees_amount )?;
        token::transfer(cpi_sol_ctx.with_signer(&[authority_seeds]),sol_to_ray )?;
        token::transfer(cpi_coin_ctx.with_signer(&[authority_seeds]),token_to_ray )?;

             
        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.user_wallet.to_account_info(),
                ctx.accounts.amm_id.to_account_info(),
                ctx.accounts.amm_open_orders.to_account_info(),
                ctx.accounts.pool_coin_token_account.to_account_info(),
                ctx.accounts.pool_pc_token_account.to_account_info(),
                ctx.accounts.pool_withdraw_queue.to_account_info(),
                ctx.accounts.pool_target_orders_account.to_account_info(),
                ctx.accounts.pool_lp_token_account.to_account_info(),
                ctx.accounts.pool_temp_lp_token_account.to_account_info(),
                ctx.accounts.serum_market.to_account_info(),
                ctx.accounts.lp_mint_address.to_account_info(),
                ctx.accounts.amm_authority.to_account_info(),
                ctx.accounts.coin_mint_address.to_account_info(),
                ctx.accounts.pc_mint_address.to_account_info(),
                ctx.accounts.campaign_authority.to_account_info(),



                ctx.accounts.rent.to_account_info(),



            ],
            &[&authority_seeds[..]]

        ).unwrap();
         
        ctx.accounts.campaign_account.succeeded = true;

            
            
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
        ctx.accounts.config_account.serum_program = ctx.accounts.serum_dex.key();
        ctx.accounts.config_account.raydium_program = ctx.accounts.raydium_dex.key();





    
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
      ctx.accounts.config_account.serum_program = ctx.accounts.serum_dex.key();
      ctx.accounts.config_account.raydium_program = ctx.accounts.raydium_dex.key();

  





  
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

#[instruction(bump: u8,hard_cap: u64,presale_buy_rate:u64,raydium_percentage : u64)]

pub struct InitializeCampaign<'info> {


    #[account(mut)]
    pub initializer: Signer<'info>,

    pub mint: Box<Account<'info, Mint>>,
    // #[account(
    //     address = Pubkey::new_from_array([6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220, 26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1]),
    // )]
    pub pc_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        token::mint = mint,
        token::authority = initializer,
    )]
    pub temp_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = initializer,
        token::mint = pc_mint,
        token::authority = initializer,
    )]
    pub pc_temp_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        
        seeds = [&mint.to_account_info().key.as_ref().to_owned()[0..9]],
        bump,
        payer = initializer,        
        space = 44+ 1 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8+ 44 + 44 + 44 + 8 + 8 + 44,
        // space= 160
        

    )]

    pub campaign_account: Box<Account<'info,CampaignAccount>>,
    #[account(
        mut,
        // constraint = initializer_deposit_token_account.amount >= (presale_buy_rate*hard_cap) / u64::pow(10, 9) + (hard_cap * raydium_percentage as u64 ) * 1000,
        // constraint = initializer_deposit_token_account.mint == *mint.to_account_info().key,
    )]
    pub initializer_deposit_token_account: Box<Account<'info, TokenAccount>>,


    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,
    pub token_program: AccountInfo<'info>,





}

#[account]
pub struct CampaignAccount {

    token_address: Pubkey,

    pc_token_address : Pubkey,

    hard_cap: u64,

    soft_cap: u64,

    max_per_wallet: u64,

    min_per_wallet: u64,

    presale_buy_rate: u64,

    start_date_timestamp: i64,

    end_date_timestamp: i64,

    total_lamports_collected: u64,

    raydium_percentage : u64,

    listing_price: u64,

    temp_token_account: Pubkey,

    pc_temp_token_account: Pubkey,


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
    pub mint: Box<Account<'info, Mint>>,

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
    

    pub buyer_account: Box<Account<'info,BuyerAccount>>,
    #[account(mut,signer)]

    pub temp_wsol : Box<Account<'info,TokenAccount>>,


    #[account(mut)]
    
    pub pc_temp_token_account : Box<Account<'info,TokenAccount>>,

    pub token_program: AccountInfo<'info>,
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
       has_one = pc_temp_token_account,
       constraint = campaign_account.token_address == *mint.to_account_info().key


    )]
    pub campaign_account: Account<'info,CampaignAccount>,

    #[account(
        mut
    )]
    pub pc_temp_token_account: Box<Account<'info,TokenAccount>>,
    #[account(
        has_one = owner
    )]

    pub owner_temp_token_account: Box<Account<'info,TokenAccount>>,


    #[account(mut)]
    pub campaign_authority : AccountInfo<'info>,

    #[account(
       mut,
       has_one = campaign_account,
       has_one = owner,
       close = owner,
       seeds = [&campaign_account.to_account_info().key.as_ref().to_owned()[0..4],&owner.to_account_info().key.as_ref().to_owned()[0..5]],
       bump = buyer_bump,



     )]
    pub buyer_account: Box<Account<'info,BuyerAccount>>,
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

    pub config_account: Box<Account<'info,ConfigAccount>>,
    pub token_program: AccountInfo<'info>,






}


#[derive(Accounts)]
#[instruction(
    campaign_bump: u8,
    config_bump:u8
)]
pub struct InitializeMarket<'info> {

    #[account(mut)]
    pub initializer: Signer<'info>,

    #[account(mut)]
    pub campaign_authority : AccountInfo<'info>,

    pub serum_program: AccountInfo<'info>,


    #[account(mut
        ,signer
    )]
    pub market: AccountInfo<'info>,

    #[account(mut
    )]
    pub requestQueue: AccountInfo<'info>,

    #[account(mut
    )]
    pub eventQueue: AccountInfo<'info>,

    #[account(mut
    )]
    pub bids: AccountInfo<'info>,

    #[account(mut
    )]
    pub asks: AccountInfo<'info>,

    #[account(mut
    )]
    pub baseVault: Box<Account<'info, TokenAccount>>,
    
    #[account(mut
    )]
    pub quoteVault: Box<Account<'info, TokenAccount>>,


    pub baseMint: Box<Account<'info, Mint>>,

    pub quoteMint: Box<Account<'info, Mint>>,
    #[account(mut,
        constraint= campaignAccount.token_address == baseMint.key(),
        constraint= campaignAccount.pc_token_address ==Pubkey::new_from_array([6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220, 26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1]),
        seeds = [&baseMint.to_account_info().key.as_ref().to_owned()[0..9]],
        bump = campaign_bump,
    )]
    pub campaignAccount: Box<Account<'info, CampaignAccount>>,
    #[account(
        mut,
        has_one = serum_program,
        seeds = [CONFIG_PDA_SEED],
        bump = config_bump,

    )]
    pub configAccount : Box<Account<'info, ConfigAccount>>,



    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,






    pub token_program: AccountInfo<'info>,





}
#[derive(Accounts)]
#[instruction(
    campaign_bump: u8,
    config_bump:u8
)]
pub struct AddLiquidity<'info> {

  #[account(mut)]
    pub user_wallet: Signer<'info>,

    pub serum_program: AccountInfo<'info>,
    pub programId : AccountInfo<'info>,



    #[account(mut)]
    pub amm_id : AccountInfo<'info>,

    pub amm_authority : AccountInfo<'info>,


    #[account(mut)]
    pub amm_open_orders : AccountInfo<'info>,

    #[account(mut,
    )]
    pub lp_mint_address : Box<Account<'info, Mint>>,

    #[account(mut)]
    pub coin_mint_address: Box<Account<'info, Mint>>,

    #[account(mut,
    
        constraint= pc_mint_address.key() ==Pubkey::new_from_array([6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220, 26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1]),

    )]
    pub pc_mint_address: Box<Account<'info, Mint>>,


    #[account(mut,


    )]
    pub pool_coin_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut,


    )]
    pub pool_pc_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub pool_withdraw_queue : AccountInfo<'info>,

    #[account(mut)]
    pub pool_target_orders_account : AccountInfo<'info>,

    #[account(
        signer,
        init,
        payer = user_wallet,
        token::mint = lp_mint_address,
        token::authority = user_wallet,
    )]
    pub pool_lp_token_account : Box<Account<'info, TokenAccount>>,


    #[account(mut,
    
    )]
    pub pool_temp_lp_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut,
        constraint= campaign_account.token_address == coin_mint_address.key(),
        has_one = temp_token_account,
        has_one = pc_temp_token_account,
        
        seeds = [&coin_mint_address.to_account_info().key.as_ref().to_owned()[0..9]],
        bump = campaign_bump,
        constraint = campaign_account.token_address == *coin_mint_address.to_account_info().key
        
    )]
    pub campaign_account: Account<'info, CampaignAccount>,

    #[account(mut)]
    pub serum_market : AccountInfo<'info>,
    #[account(mut)]

    pub campaign_authority: AccountInfo<'info>,
    #[account(mut)]
    pub temp_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub pc_temp_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: AccountInfo<'info>,
    
    #[account(
        mut,
        has_one = serum_program,
        constraint = configAccount.raydium_program == programId.key(),
        seeds = [CONFIG_PDA_SEED],
        bump = config_bump,
        has_one = to_address
   
    )]
    pub configAccount : Box<Account<'info, ConfigAccount>>,
    pub to_address:  AccountInfo<'info>,

    pub system_program: Program<'info, System>,




    pub rent: Sysvar<'info, Rent>,












}

#[derive(Accounts)]
#[instruction(
    bump: u8,
)]
pub struct InitConfigAccount<'info> {

    pub to_address:  AccountInfo<'info>,

    #[account(mut,signer)]

    pub owner: AccountInfo<'info>,

    #[account
    (
        init,
        payer = owner,
        space = 44 + 8 + 44 +2,
        seeds = [CONFIG_PDA_SEED],
        bump


    
    )]

    pub config_account: Account<'info,ConfigAccount>,
    pub serum_dex: AccountInfo<'info>,
    pub raydium_dex: AccountInfo<'info>,


    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,


    
}

#[derive(Accounts)]
#[instruction(
    bump: u8,
)]
pub struct EditConfigAccount<'info> {

    pub to_address:  AccountInfo<'info>,

    #[account(mut,signer)]

    pub owner: AccountInfo<'info>,
    pub serum_dex: AccountInfo<'info>,
    pub raydium_dex: AccountInfo<'info>,

    #[account
    (

        seeds = [CONFIG_PDA_SEED],
        bump,
        has_one = owner


    
    )]

    pub config_account: Account<'info,ConfigAccount>,



    
}


#[account]
pub struct ConfigAccount {

    to_address: Pubkey,

    fee: u64,

    owner: Pubkey,
    serum_program: Pubkey,
    raydium_program: Pubkey,

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
    pub buyer_account: Box<Account<'info,BuyerAccount>>,

    #[account(mut)]
    pub temp_token_account: Box<Account<'info, TokenAccount>>,
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
            authority: self.initializer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.temp_token_account.to_account_info().clone(),
            current_authority: self.initializer.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
    fn into_set_pc_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.pc_temp_token_account.to_account_info().clone(),
            current_authority: self.initializer.to_account_info().clone(),
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
impl<'info> AddLiquidity<'info> {
    

    fn into_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.pool_lp_token_account.to_account_info().clone(),
            current_authority: self.user_wallet.to_account_info().clone(),
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
    CongfigInitialized,
    #[msg("Invalid Token Amount")]
    InvalidTokenAmount



}
