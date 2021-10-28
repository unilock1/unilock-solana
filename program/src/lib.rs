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
        max_per_wallet: u128,
    
        #[allow(dead_code)] // not dead code..
        min_per_wallet: u128,
        #[allow(dead_code)] // not dead code..
        presale_buy_rate: u128,

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
    WithdrawFunds {},
    DistributeFunds {},
    CreateConfigAccount {fee : u128},
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
    max_per_wallet: u128,

    #[allow(dead_code)] // not dead code..
    min_per_wallet: u128,

    #[allow(dead_code)] // not dead code..
    presale_buy_rate: u128,

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

    #[allow(dead_code)] // not dead code..
    succeeded: bool,

    #[allow(dead_code)] // not dead code..
    owner: Pubkey,
    
    
}
#[derive(Clone, BorshSerialize, BorshDeserialize)]

pub struct ConfigAccount {

    #[allow(dead_code)] // not dead code..
    to_address: Pubkey,
    #[allow(dead_code)] // not dead code..
    fee: u128,
    #[allow(dead_code)] // not dead code..
    owner: Pubkey,
    #[allow(dead_code)] // not dead code..
    initialized: bool,

    
    
}
#[derive(Clone, BorshSerialize, BorshDeserialize)]

pub struct CreateAccount {

    #[allow(dead_code)] // not dead code..
    fee: u128,


    
    
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
    max_per_wallet: u128,

    #[allow(dead_code)] // not dead code..
    min_per_wallet: u128,

    #[allow(dead_code)] // not dead code..
    presale_buy_rate: u128,

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
                    max_per_wallet : deserialized_data.max_per_wallet,
                    min_per_wallet : deserialized_data.min_per_wallet,
                    presale_buy_rate: deserialized_data.presale_buy_rate,
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
            3 => Self::WithdrawFunds {},
            4 => Self::DistributeFunds {},
            5 => {
                let deserialized_data: CreateAccount =
                BorshDeserialize::try_from_slice(&mut &rest[..]).unwrap();
            Self::CreateConfigAccount {
                fee: deserialized_data.fee,
            }

            }
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

    let signer_account = next_account_info(accounts_iter)?;

   
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
            max_per_wallet,
            min_per_wallet,
            presale_buy_rate,
            start_date_timestamp,
            end_date_timestamp,
        } => {

            let campaign_account = next_account_info(accounts_iter)?;

            let mut account_data = campaign_account.try_borrow_mut_data()?;
        
            if campaign_account.owner != program_id {
                msg!("Account is not owned by this program");
                return Err(ProgramError::InvalidInstructionData);
            }
            msg!("Create Campaign");

            let mint_token_address = next_account_info(accounts_iter)?;
            let temp_token_account = next_account_info(accounts_iter)?;
            let token_program = next_account_info(accounts_iter)?;
            let (pda, _bump_seed) = Pubkey::find_program_address(&[b"contract"], program_id);

            let temp_token_account_info =
                TokenAccount::unpack_from_slice(&temp_token_account.data.borrow())?;

            let amount_to_be_sold = (hard_cap * presale_buy_rate) / u128::pow(10, 9);


            let total_amount = amount_to_be_sold ;
            let token_key = (*mint_token_address.key).as_ref().to_owned();
            let camp_seed = (&token_key[0..9]).iter().map(|&c| c as char).collect::<String>();

            let expected_campaign_account_key =
                Pubkey::create_with_seed(signer_account.key, &camp_seed, program_id).unwrap();


            if expected_campaign_account_key != *campaign_account.key {

                msg!("Campaign account is doesn't match the expected");
                return Err(ProgramError::InvalidInstructionData);

            }
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
                max_per_wallet,
                min_per_wallet,
                presale_buy_rate,
                start_date_timestamp,
                end_date_timestamp,
                total_lamports_collected: 0,
                temp_token_account: *temp_token_account.key,
                initialized: true,
                succeeded : false,
                owner : *signer_account.key
            };

            to_serialize.serialize(&mut &mut account_data[..])?;
        }
        AccountInstruction::BuyToken { lamports_quantity } => {


            let campaign_account = next_account_info(accounts_iter)?;

            let mut account_data = campaign_account.try_borrow_mut_data()?;
        
            if campaign_account.owner != program_id {
                msg!("Account is not owned by this program");
                return Err(ProgramError::InvalidInstructionData);
            }

            msg!("Buy Token");


            let buyer_account = next_account_info(accounts_iter)?;
            let temp_buy_account = next_account_info(accounts_iter)?;

            let clock = Clock::get()?;

            let campaign_key = (*campaign_account.key).as_ref().to_owned();

            let seed = (&campaign_key[0..9]).iter().map(|&c| c as char).collect::<String>();


            let expected_buyer_account_key =
                Pubkey::create_with_seed(signer_account.key, &seed, program_id).unwrap();
            if expected_buyer_account_key != *buyer_account.key {

                msg!("seed {}",expected_buyer_account_key);
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

        
           

            if !is_live(
                campaign_account_data_res.clone(),
                clock.unix_timestamp,
            ) || is_failed(
                campaign_account_data_res.clone(),
                clock.unix_timestamp,
            ) {

                msg!("Current timestamp {}",clock.unix_timestamp);
                msg!("Campaign is not live");
                return Err(ProgramError::InvalidInstructionData);
            }

            if buyer_account_data_des.initialized != true {
                buyer_account_data_des = BuyerAccount {
                    campaign_account: *campaign_account.key,
                    contributed_lamports: 0,
                    initialized: true,
                    initializer: *signer_account.key,
                    claimed: false,
                }
            }
            if buyer_account_data_des.initializer != *signer_account.key {
                msg!("Buyer account is not owned by the signer");
                return Err(ProgramError::InvalidInstructionData);
            };

            if campaign_account_data_res.hard_cap < campaign_account_data_res.total_lamports_collected + lamports_quantity {

                msg!("Can't contribute more than hardcap");
                return Err(ProgramError::InvalidInstructionData);
            }

            if campaign_account_data_res.max_per_wallet < buyer_account_data_des.contributed_lamports + lamports_quantity {
                msg!("Can't buy more than you are allowed to");
                return Err(ProgramError::InvalidInstructionData);
            }
            if (campaign_account_data_res.min_per_wallet > lamports_quantity) && ( campaign_account_data_res.min_per_wallet < campaign_account_data_res.hard_cap - campaign_account_data_res.total_lamports_collected) {
                msg!("Can't buy less than you are required to");
                return Err(ProgramError::InvalidInstructionData);
            }


            if *campaign_account.key != buyer_account_data_des.campaign_account{

                msg!("Buyer account is not owned by campaign");
                return Err(ProgramError::InvalidInstructionData);

            }

            buyer_account_data_des.contributed_lamports += lamports_quantity;

            campaign_account_data_res.total_lamports_collected += lamports_quantity;

            **temp_buy_account.try_borrow_mut_lamports()? -= lamports_quantity as u64;
            **campaign_account.try_borrow_mut_lamports()? += lamports_quantity as u64;

            buyer_account_data_des.serialize(&mut &mut buyer_account_data[..])?;

            campaign_account_data_res.serialize(&mut &mut account_data[..])?;
        }
        AccountInstruction::ClaimToken {} => {
            let campaign_account = next_account_info(accounts_iter)?;

            let mut account_data = campaign_account.try_borrow_mut_data()?;
        
            if campaign_account.owner != program_id {
                msg!("Account is not owned by this program");
                return Err(ProgramError::InvalidInstructionData);
            }
            msg!("ClaimToken");


            
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

            let seed = (&campaign_key[0..9]).iter().map(|&c| c as char).collect::<String>();

            let expected_buyer_account_key =
                Pubkey::create_with_seed(signer_account.key, &seed, program_id).unwrap();

            if associated_token_account_info.owner != *signer_account.key {
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
                clock.unix_timestamp,
            ) || is_failed(
                campaign_account_data_res.clone(),
                clock.unix_timestamp,
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

            // buyer_account_data_des.claimed = true;

            // buyer_account_data_des.serialize(&mut &mut buyer_account_data[..])?;
            **signer_account.try_borrow_mut_lamports()? = signer_account.lamports().checked_add(buyer_account.lamports()).ok_or(ProgramError::InvalidInstructionData)?;
            **buyer_account.try_borrow_mut_lamports()? = 0;
            *buyer_account_data = &mut [];

        }
        AccountInstruction::WithdrawFunds {} => {
            let campaign_account = next_account_info(accounts_iter)?;

            let mut account_data = campaign_account.try_borrow_mut_data()?;
        
            if campaign_account.owner != program_id {
                msg!("Account is not owned by this program");
                return Err(ProgramError::InvalidInstructionData);
            }

            msg!("WithdrawFunds");


            let buyer_account = next_account_info(accounts_iter)?;
            let config_account = next_account_info(accounts_iter)?;
            let to_fee_account = next_account_info(accounts_iter)?;

            

            let mut config_account_data = config_account.try_borrow_mut_data()?;

            let  config_account_data_des: ConfigAccount =
            BorshDeserialize::deserialize(&mut &config_account_data[..]).unwrap();
            
            let mut buyer_account_data = buyer_account.try_borrow_mut_data()?;
            let mut buyer_account_data_des: BuyerAccount =
            BorshDeserialize::deserialize(&mut &buyer_account_data[..]).unwrap();
            

            let campaign_account_data_res: CampaignAccount =
                BorshDeserialize::deserialize(&mut &account_data[..]).unwrap();
            

            let clock = Clock::get()?;

            let campaign_key = (*campaign_account.key).as_ref().to_owned();

            let seed = (&campaign_key[0..9]).iter().map(|&c| c as char).collect::<String>();
            let config_seed ="config".to_owned();
            let owner_key = [206, 183, 147, 105, 49, 82, 73, 110, 176, 156, 216, 99, 202, 54, 75, 239, 27, 254, 44, 83, 4, 7, 122, 34, 14, 36, 95, 119, 123, 229, 28, 188];

            let expected_config_account = Pubkey::create_with_seed(&Pubkey::new_from_array(owner_key), &config_seed, program_id).unwrap();
        


            let expected_buyer_account_key =
                Pubkey::create_with_seed(signer_account.key, &seed, program_id).unwrap();

    
            if expected_buyer_account_key != *buyer_account.key {
                msg!("Buyer account doesn't meet the required pattern");
                return Err(ProgramError::InvalidInstructionData);
            };
            if config_account_data_des.to_address != *to_fee_account.key {
                msg!("Config to_fee is not as expected");
                return Err(ProgramError::InvalidInstructionData);
            };
            if expected_config_account != *config_account.key {
                msg!("Config account doesn't meet the required pattern");
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
    
            if !is_failed(
                campaign_account_data_res.clone(),
                clock.unix_timestamp,
            ) {
                msg!("Can't withdraw fund");
                return Err(ProgramError::InvalidInstructionData);
            }

            if buyer_account_data_des.claimed == true {
                msg!("Already claimed");
                return Err(ProgramError::InvalidInstructionData);
            }

            let to_withdraw_amount = (buyer_account_data_des.contributed_lamports * config_account_data_des.fee) / 1000;

            let to_fees_amount = buyer_account_data_des.contributed_lamports - to_withdraw_amount;
            

            

            **campaign_account.try_borrow_mut_lamports()? -= buyer_account_data_des.contributed_lamports as u64;
            **signer_account.try_borrow_mut_lamports()? += to_withdraw_amount as u64;
            **to_fee_account.try_borrow_mut_lamports()? += to_fees_amount as u64; 
           
            **signer_account.try_borrow_mut_lamports()? = signer_account.lamports().checked_add(buyer_account.lamports()).ok_or(ProgramError::InvalidInstructionData)?;
            **buyer_account.try_borrow_mut_lamports()? = 0;
            *buyer_account_data = &mut [];


            
        }
        AccountInstruction::DistributeFunds {} =>{
            let campaign_account = next_account_info(accounts_iter)?;

            let mut account_data = campaign_account.try_borrow_mut_data()?;
        
            if campaign_account.owner != program_id {
                msg!("Account is not owned by this program");
                return Err(ProgramError::InvalidInstructionData);
            }


            msg!("Distribute Funds");




            let campaign_owner = next_account_info(accounts_iter)?;

            let config_account = next_account_info(accounts_iter)?;
            let to_fee_account = next_account_info(accounts_iter)?;
            let owner_key = [206, 183, 147, 105, 49, 82, 73, 110, 176, 156, 216, 99, 202, 54, 75, 239, 27, 254, 44, 83, 4, 7, 122, 34, 14, 36, 95, 119, 123, 229, 28, 188];
            

            

            let  config_account_data = config_account.try_borrow_mut_data()?;

            let  config_account_data_des: ConfigAccount =
            BorshDeserialize::deserialize(&mut &config_account_data[..]).unwrap();

            let mut campaign_account_data_res: CampaignAccount =
            BorshDeserialize::deserialize(&mut &account_data[..]).unwrap();




            let clock = Clock::get()?;

            let config_seed ="config".to_owned();

            let expected_config_account = Pubkey::create_with_seed(&Pubkey::new_from_array(owner_key), &config_seed, program_id).unwrap();
            if campaign_account_data_res.owner != *campaign_owner.key {
                msg!("Invalid campaign owner");
                return Err(ProgramError::InvalidInstructionData);
            }
            if config_account_data_des.to_address != *to_fee_account.key {
                msg!("Config to_fee is not as expected");
                return Err(ProgramError::InvalidInstructionData);
            };
            if expected_config_account != *config_account.key {
                msg!("Config account doesn't meet the required pattern");
                return Err(ProgramError::InvalidInstructionData);
            };
            if is_live(
                campaign_account_data_res.clone(),
                clock.unix_timestamp,
            ) || is_failed(
                campaign_account_data_res.clone(),
                clock.unix_timestamp,
            ) {

                msg!("Current timestamp {}",clock.unix_timestamp);
                msg!("Campaign is not live");
                return Err(ProgramError::InvalidInstructionData);
            }

            if campaign_account_data_res.succeeded == true {
                msg!("Campaign already succeeded");
                return Err(ProgramError::InvalidInstructionData);
            }
            let  to_distribute_amount= campaign_account_data_res.total_lamports_collected * config_account_data_des.fee / 1000 ;

            let to_fees_amount = campaign_account_data_res.total_lamports_collected as u64  - to_distribute_amount as u64;


            **campaign_account.try_borrow_mut_lamports()? -= campaign_account_data_res.total_lamports_collected as u64;

            **campaign_owner.try_borrow_mut_lamports()? += to_distribute_amount as u64;
            **to_fee_account.try_borrow_mut_lamports()? += to_fees_amount as u64; 

            campaign_account_data_res.succeeded = true;
            campaign_account_data_res.serialize(&mut &mut account_data[..])?;







        },
        AccountInstruction::CreateConfigAccount{fee}=>{
            let config_account = next_account_info(accounts_iter)?;
            let config_seed ="config".to_owned();

            let mut config_account_data = config_account.try_borrow_mut_data()?;
            let owner_key = [206, 183, 147, 105, 49, 82, 73, 110, 176, 156, 216, 99, 202, 54, 75, 239, 27, 254, 44, 83, 4, 7, 122, 34, 14, 36, 95, 119, 123, 229, 28, 188];

            let expected_config_account = Pubkey::create_with_seed(&Pubkey::new_from_array(owner_key), &config_seed, program_id).unwrap();


            if expected_config_account != *config_account.key {
                msg!("Config account doesn't meet the required pattern");
                return Err(ProgramError::InvalidInstructionData);
            };


            let config_data_des: ConfigAccount =
                BorshDeserialize::deserialize(&mut &config_account_data[..]).unwrap();

            if config_data_des.initialized {
                msg!("Config already initialized");
                return Err(ProgramError::InvalidInstructionData);
            };

            let to_serialize: ConfigAccount = ConfigAccount {
                to_address :*signer_account.key,
                fee: fee,
                owner : *signer_account.key,
                initialized : true,
            };

            to_serialize.serialize(&mut &mut config_account_data[..])?;

        },
        AccountInstruction::InvalidInst {} => {
            return Err(ProgramError::BorshIoError(String::from(
                "Invalid instruction",
            )))
        }
    }
    fn is_live(campaign: CampaignAccount, current_time: i64) -> bool {
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

    fn is_failed(campaign: CampaignAccount, current_time: i64) -> bool {
        if (campaign.total_lamports_collected < campaign.soft_cap)
            && (current_time > campaign.end_date_timestamp)
        {
            return true;
        }else{
            return false;

        }
    }

    Ok(())
}
