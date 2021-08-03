use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use serde_json::Result;

use std::str;

use spl_token::state::Account as TokenAccount;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
    pubkey::Pubkey,
    sysvar::{
        self, clock::Clock, epoch_schedule::EpochSchedule, fees::Fees, instructions,
        recent_blockhashes::RecentBlockhashes, rent::Rent, slot_hashes::SlotHashes,
        slot_history::SlotHistory, stake_history::StakeHistory, Sysvar,
    },
};
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum AccountInstruction {
    CreateCampaign {
        hard_cap: u128,
        #[allow(dead_code)] // not dead code..
        soft_cap: u128,

        #[allow(dead_code)] // not dead code..
        presale_buy_rate: u128,

        #[allow(dead_code)] // not dead code..
        exchange_percentage: u128,

        #[allow(dead_code)] // not dead code..
        presale_listing_exchange_rate: u128,

        #[allow(dead_code)] // not dead code..
        start_date_timestamp: i64,

        #[allow(dead_code)] // not dead code..
        end_date_timestamp: i64,
    },
    BuyToken {
        #[allow(dead_code)] // not dead code..
        lamports_quantity: u128,
    },
    ClaimToken {},
    InvalidInst {},
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]

pub struct CampaignAccount {
    // PRESALE DATA //
    #[allow(dead_code)] // not dead code..
    token_address: Pubkey,
    #[allow(dead_code)] // not dead code..
    hard_cap: u128,
    #[allow(dead_code)] // not dead code..
    soft_cap: u128,

    #[allow(dead_code)] // not dead code..
    presale_buy_rate: u128,

    #[allow(dead_code)] // not dead code..
    exchange_percentage: u128,

    #[allow(dead_code)] // not dead code..
    presale_listing_exchange_rate: u128,

    #[allow(dead_code)] // not dead code..
    start_date_timestamp: i64,

    #[allow(dead_code)] // not dead code..
    end_date_timestamp: i64,

    #[allow(dead_code)] // not dead code..
    total_lamports_collected: u128,

    #[allow(dead_code)] // not dead code..
    temp_token_account: Pubkey,

    #[allow(dead_code)] // not dead code..
    initialized: bool,
}
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]

pub struct BuyerAccount {
    // PRESALE DATA //
    #[allow(dead_code)] // not dead code..
    campaign_account: Pubkey,

    #[allow(dead_code)] // not dead code..
    contributed_lamports: u128,

    #[allow(dead_code)] // not dead code..
    initialized: bool,

    #[allow(dead_code)] // not dead code..
    initializer: Pubkey,

    #[allow(dead_code)] // not dead code..
    claimed: bool,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq, Deserialize)]

pub struct CreatedCampaign {
    hard_cap: u128,
    #[allow(dead_code)] // not dead code..
    soft_cap: u128,

    #[allow(dead_code)] // not dead code..
    presale_buy_rate: u128,

    #[allow(dead_code)] // not dead code..
    exchange_percentage: u128,

    #[allow(dead_code)] // not dead code..
    presale_listing_exchange_rate: u128,

    #[allow(dead_code)] // not dead code..
    start_date_timestamp: i64,

    #[allow(dead_code)] // not dead code..
    end_date_timestamp: i64,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]

pub struct BuyToken {
    #[allow(dead_code)] // not dead code..
    lamports_quantity: u128,
}

impl AccountInstruction {
    /// Unpacks a byte buffer into a [TokenInstruction](enum.TokenInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self> {
        use ProgramError::InvalidInstructionData;

        let (&tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)
            .unwrap();
        Ok(match tag {
            0 => {
                let deserialized_data: CreatedCampaign =
                    BorshDeserialize::try_from_slice(&mut &rest[..]).unwrap();
                Self::CreateCampaign {
                    hard_cap: deserialized_data.hard_cap,
                    soft_cap: deserialized_data.soft_cap,
                    presale_buy_rate: deserialized_data.presale_buy_rate,
                    exchange_percentage: deserialized_data.exchange_percentage,
                    presale_listing_exchange_rate: deserialized_data.presale_listing_exchange_rate,
                    start_date_timestamp: deserialized_data.start_date_timestamp,
                    end_date_timestamp: deserialized_data.end_date_timestamp,
                }
            }
            1 => {

                let deserialized_data: BuyToken =
                    BorshDeserialize::try_from_slice(&mut &rest[..]).unwrap();
                Self::BuyToken {
                    lamports_quantity: deserialized_data.lamports_quantity,
                }
            }
            2 => Self::ClaimToken {},
            _ => Self::InvalidInst {},
        })
    }
}

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
pub fn process_instruction(
    program_id: &Pubkey, 
    accounts: &[AccountInfo],
    instruction_data: &[u8], 
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let campaign_account = next_account_info(accounts_iter)?;
    let signer_account = next_account_info(accounts_iter)?;

    let mut account_data = campaign_account.try_borrow_mut_data()?;

    if campaign_account.owner != program_id {
        msg!("Account is not owned by this program");
        return Err(ProgramError::InvalidInstructionData);
    }
    if !signer_account.is_signer {
        msg!("You are not a signer");
        return Err(ProgramError::InvalidInstructionData);
    };

    let mut deserialized_data: AccountInstruction =
        AccountInstruction::unpack(&instruction_data).unwrap();

    match deserialized_data {
        AccountInstruction::CreateCampaign {
            hard_cap,
            soft_cap,
            presale_buy_rate,
            exchange_percentage,
            presale_listing_exchange_rate,
            start_date_timestamp,
            end_date_timestamp,
        } => {
            msg!("Create Campaign");

            let mint_token_address = next_account_info(accounts_iter)?;
            let temp_token_account = next_account_info(accounts_iter)?;
            let token_program = next_account_info(accounts_iter)?;
            let (pda, _bump_seed) = Pubkey::find_program_address(&[b"contract"], program_id);

            let temp_token_account_info =
                TokenAccount::unpack_from_slice(&temp_token_account.data.borrow())?;

            let amount_to_be_sold = (hard_cap * presale_buy_rate) / u128::pow(10, 9);
            let amount_to_add_to_exchange = (((hard_cap * exchange_percentage) / 100)
                * presale_listing_exchange_rate)
                / u128::pow(10, 9);

            let total_amount = amount_to_add_to_exchange + amount_to_be_sold;

            if temp_token_account_info.amount as u128 != total_amount {
                                                              
                msg!("Required amount doesn't match Account amount");
                return Err(ProgramError::InvalidInstructionData);

            }
            if temp_token_account_info.mint != *mint_token_address.key {
                                                                                
                msg!("Temp account mint doesn't match the token mint");
                return Err(ProgramError::InvalidInstructionData);
            }

            let owner_change_ix = spl_token::instruction::set_authority(
                token_program.key,
                temp_token_account.key,
                Some(&pda),
                spl_token::instruction::AuthorityType::AccountOwner,
                signer_account.key,
                &[&signer_account.key],
            )?;

            invoke(
                &owner_change_ix,
                &[
                    temp_token_account.clone(),
                    signer_account.clone(),
                    token_program.clone(),
                ],
            )?;

            //  spl_token::instruction::

            // Begin Create account

            // let  seed = "heyll".to_owned();
            // let  s = &seed[..].as_ref();
            // let program_key = Pubkey::create_with_seed(signer_account.key,s,program_id).unwrap();

            // End create account
            // clock;
            // sysvar::epoch_schedule::id().log();

            let mut campaign_data: CampaignAccount =
                BorshDeserialize::deserialize(&mut &account_data[..]).unwrap();

            if campaign_data.initialized {
                                                                
                msg!("Campaign already initialized");
                return Err(ProgramError::InvalidInstructionData);
            };

            let to_serialize: CampaignAccount = CampaignAccount {
                token_address: *mint_token_address.key,
                hard_cap: hard_cap,
                soft_cap,
                presale_buy_rate,
                exchange_percentage,
                presale_listing_exchange_rate,
                start_date_timestamp,
                end_date_timestamp,
                total_lamports_collected: 0,
                temp_token_account: *temp_token_account.key,
                initialized: true,
            };

            to_serialize.serialize(&mut &mut account_data[..])?;
        }
        AccountInstruction::BuyToken { lamports_quantity } => {
            let buyer_account = next_account_info(accounts_iter)?;
            let temp_buy_account = next_account_info(accounts_iter)?;

     
            let clock = Clock::get()?;

            let campaign_key = (*campaign_account.key).as_ref().to_owned();

            let seed = String::from_utf8_lossy(&campaign_key[0..9]);

            let s = &seed[..].as_ref();

            let expected_buyer_account_key =
                Pubkey::create_with_seed(signer_account.key,s, program_id).unwrap();
            if expected_buyer_account_key != *buyer_account.key {
                                
                msg!("Buyer account doesn't meet the required pattern");
                return Err(ProgramError::InvalidInstructionData);
            };
            if buyer_account.owner != program_id {
                                                
                msg!("Buyer account is not owned by the program");
                return Err(ProgramError::InvalidInstructionData);
            };



            let mut buyer_account_data = buyer_account.try_borrow_mut_data()?;

            let mut buyer_account_data_des: BuyerAccount =
                BorshDeserialize::deserialize(&mut &buyer_account_data[..]).unwrap();

            let mut campaign_account_data_res: CampaignAccount =
                BorshDeserialize::deserialize(&mut &account_data[..]).unwrap();

            // if !isLive(
            //     campaign_account_data_res.clone(),
            //     clock.epoch_start_timestamp,
            // ) || isFailed(
            //     campaign_account_data_res.clone(),
            //     clock.epoch_start_timestamp,
            // ) {
            //     msg!("Campaign is not live");
            //     return Err(ProgramError::IncorrectProgramId);
            // }

            if buyer_account_data_des.initialized != true {
                buyer_account_data_des = BuyerAccount {
                    campaign_account: *campaign_account.key,
                    contributed_lamports: 0,
                    initialized: true,
                    initializer: *signer_account.key,
                    claimed: false,
                }
            }

    

            buyer_account_data_des.contributed_lamports += lamports_quantity;

            campaign_account_data_res.total_lamports_collected += lamports_quantity;

            **temp_buy_account.try_borrow_mut_lamports()? -= lamports_quantity as u64;
            **campaign_account.try_borrow_mut_lamports()? += lamports_quantity as u64;


            buyer_account_data_des.serialize(&mut &mut buyer_account_data[..])?;

            campaign_account_data_res.serialize(&mut &mut account_data[..])?;
        }
        AccountInstruction::ClaimToken {} => {
            let associated_token_account = next_account_info(accounts_iter)?;
            let buyer_account = next_account_info(accounts_iter)?;
            let temp_token_account = next_account_info(accounts_iter)?;
            let pda_account = next_account_info(accounts_iter)?;
            let token_program = next_account_info(accounts_iter)?;
            let mut buyer_account_data = buyer_account.try_borrow_mut_data()?;

            let clock = Clock::get()?;

            let associated_token_account_info =
                TokenAccount::unpack_from_slice(&associated_token_account.data.borrow())?;


            let campaign_account_data_res: CampaignAccount =
                BorshDeserialize::deserialize(&mut &account_data[..]).unwrap();


            let mut buyer_account_data_des: BuyerAccount =
                BorshDeserialize::deserialize(&mut &buyer_account_data[..]).unwrap();


            let campaign_key = (*campaign_account.key).as_ref().to_owned();

            let seed = String::from_utf8_lossy(&campaign_key[0..9]);

            let s = &seed[..].as_ref();

            let expected_buyer_account_key =
                Pubkey::create_with_seed(signer_account.key, s, program_id).unwrap();

            if associated_token_account_info.owner != *buyer_account.key {
                
                msg!("Can't match given token account");
                return Err(ProgramError::InvalidInstructionData);
            };    
            if expected_buyer_account_key != *buyer_account.key {

                msg!("Buyer account doesn't meet the required pattern");
                return Err(ProgramError::InvalidInstructionData);
            };
            if *campaign_account.key != buyer_account_data_des.campaign_account {

                msg!("Unable to match campaign account for the entered buyer account");
                return Err(ProgramError::InvalidInstructionData);

            };
            if buyer_account.owner != program_id {

                msg!("Buyer account is not owned by the program");
                return Err(ProgramError::InvalidInstructionData);

            };
            if buyer_account_data_des.claimed == true {

                msg!("You can't claim twice");
                return Err(ProgramError::InvalidInstructionData);

            }


            if is_live(
                campaign_account_data_res.clone(),
                clock.epoch_start_timestamp,
            ) || is_failed(
                campaign_account_data_res.clone(),
                clock.epoch_start_timestamp,
            ) {

                msg!("Can't withdraw tokens");
                return Err(ProgramError::InvalidInstructionData);
            }

            let claimable_amount = (buyer_account_data_des.contributed_lamports
                * campaign_account_data_res.presale_buy_rate)
                / u128::pow(10, 9);

            let (pda, _bump_seed) = Pubkey::find_program_address(&[b"contract"], program_id);

            if pda != *pda_account.key {
                
                msg!("Wrong PDA");
                return Err(ProgramError::InvalidInstructionData);
            };

            let transfer_to_taker_ix = spl_token::instruction::transfer(
                token_program.key,
                &*temp_token_account.key,
                associated_token_account.key,
                &pda,
                &[&pda],
                claimable_amount as u64,
            )?;

            invoke_signed(
                &transfer_to_taker_ix,
                &[
                    temp_token_account.clone(),
                    associated_token_account.clone(),
                    pda_account.clone(),
                    token_program.clone(),
                ],
                &[&[&b"contract"[..], &[_bump_seed]]],
            )?;

            buyer_account_data_des.claimed = true;

            buyer_account_data_des.serialize(&mut &mut buyer_account_data[..])?;
        },
        AccountInstruction::InvalidInst {} => return Err(ProgramError::BorshIoError(String::from("Invalid instruction"))),

    }
    fn is_live(campaign: CampaignAccount, current_time: i64) -> bool {
        if campaign.total_lamports_collected >= campaign.hard_cap {
            return false;
        }
        if (current_time > campaign.end_date_timestamp)
            || (current_time < campaign.start_date_timestamp)
        {
            return false;
        }
        return true;
    }

    fn is_failed(campaign: CampaignAccount, current_time: i64) -> bool {
        if (campaign.total_lamports_collected < campaign.soft_cap)
            && (current_time > campaign.end_date_timestamp)
        {
            return true;
        }
        return false;
    }

    Ok(())
}
