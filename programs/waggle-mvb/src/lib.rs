use anchor_lang::{ prelude::*, solana_program::sysvar::{ slot_hashes, SysvarId } };
use anchor_spl::{
  associated_token::AssociatedToken,
  metadata::mpl_token_metadata::EDITION_MARKER_BIT_SIZE,
  metadata::{
    create_master_edition_v3,
    create_metadata_accounts_v3,
    mint_new_edition_from_master_edition_via_token,
    thaw_delegated_account,
    set_and_verify_sized_collection_item,
    freeze_delegated_account,
    mpl_token_metadata::{ programs::MPL_TOKEN_METADATA_ID, accounts::Edition },
    CreateMasterEditionV3,
    CreateMetadataAccountsV3,
    Metadata,
    MintNewEditionFromMasterEditionViaToken,
    SetAndVerifySizedCollectionItem,
    mpl_token_metadata::types::DataV2,
    MetadataAccount,
    FreezeDelegatedAccount,
    ThawDelegatedAccount,
  },
  token::{ mint_to, Mint, MintTo, Token, TokenAccount, Transfer, transfer, approve, revoke },
};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use arrayref::array_ref;
use std::mem::size_of;

pub mod merkle_proof;

declare_id!("6a2VeLtBtKAYu5ftup9o9uLEvfySdEzBhSDukHFZaopu");

pub const MPL_PREFIX: &str = "metadata";
pub const MPL_EDITION: &str = "edition";
pub const STATE: &str = "state";
pub const MVB: &str = "mvb";
pub const MINT: &str = "mint";
pub const LOCK: &str = "lock";
pub const COLLECTION: &str = "WMC";

#[program]
pub mod waggle_mvb {
  use anchor_spl::metadata::{sign_metadata, SignMetadata};

use super::*;

  pub fn init_user(_ctx: Context<InitUser>) -> Result<()> {
    let mut user_account = _ctx.accounts.user_account.load_init()?;
    user_account.authority = _ctx.accounts.authority.key();
    Ok(())
  }

  pub fn init_mvb_user(_ctx: Context<InitMvbUser>, _symbol: String) -> Result<()> {
    let mut mvb_user_account = _ctx.accounts.mvb_user_account.load_init()?;
    mvb_user_account.authority = _ctx.accounts.authority.key();
    mvb_user_account.mvb = _ctx.accounts.mvb_account.key();
    Ok(())
  }

  pub fn create_state(
    _ctx: Context<CreateState>,
    _total_supply: u64,
    _user_max_mint: u64,
    _lock_duration: i64,
    _honey_drop_cycle: i64,
    _mint_end_time: i64,
    _drop_harvest_end_time: i64,
    _honey_drops: [u64; 4]
  ) -> Result<()> {
    let state = &mut _ctx.accounts.state.load_init()?;

    state.authority = _ctx.accounts.authority.key();
    state.total_supply = _total_supply;
    state.user_max_mint = _user_max_mint;
    state.lp_mint = _ctx.accounts.lp_mint.key();
    state.lp_vault = _ctx.accounts.lp_vault.key();
    state.usd_mint = _ctx.accounts.usd_mint.key();
    state.wag_mint = _ctx.accounts.wag_mint.key();
    state.lp_usd_vault = _ctx.accounts.lp_usd_vault.key();
    state.lp_wag_vault = _ctx.accounts.lp_wag_vault.key();
    state.lock_duration = _lock_duration;
    state.honey_drop_cycle = _honey_drop_cycle;
    state.mint_end_time = _mint_end_time;
    state.honey_drop_remains = _honey_drops;
    state.service = _ctx.accounts.service.key();

    Ok(())
  }

  pub fn set_state_config(
    _ctx: Context<SetState>,
    _total_supply: Option<u64>,
    _user_max_mint: Option<u64>,
    _lock_duration: Option<i64>,
    _honey_drop_cycle: Option<i64>,
    _mint_end_time: Option<i64>,
    _drop_harvest_end_time: Option<i64>,
    _honey_drops: Option<[u64; 4]>
  ) -> Result<()> {
    let state = &mut _ctx.accounts.state.load_mut()?;

    if _total_supply.is_some() {
      state.total_supply = _total_supply.unwrap();
    }
    if _user_max_mint.is_some() {
      state.user_max_mint = _user_max_mint.unwrap();
    }
    if _lock_duration.is_some() {
      state.lock_duration = _lock_duration.unwrap();
    }
    if _honey_drop_cycle.is_some() {
      state.honey_drop_cycle = _honey_drop_cycle.unwrap();
    }
    if _mint_end_time.is_some() {
      state.mint_end_time = _mint_end_time.unwrap();
    }
    if _drop_harvest_end_time.is_some() {
      state.harvest_drop_end_time = _drop_harvest_end_time.unwrap();
    }
    if _honey_drops.is_some() {
      state.honey_drop_remains = _honey_drops.unwrap();
    }

    Ok(())
  }

  pub fn mint_master_nft(
    _ctx: Context<MintMasterNft>,
    _name: String,
    _symbol: String,
    _uri: String,
    _total_supply: u64,
    _lp_value_price: u64
  ) -> Result<()> {
    let state = _ctx.accounts.state.load()?;
    let mvb_account = &mut _ctx.accounts.mvb_account.load_init()?;

    mvb_account.mint = _ctx.accounts.mint.key();
    mvb_account.total_supply = _total_supply;
    mvb_account.lp_value_price = _lp_value_price;

    // create mint account
    let seeds = &[STATE.as_bytes(), &[_ctx.bumps.state]];
    let signer = &[&seeds[..]];

    mint_to(
      CpiContext::new_with_signer(
        _ctx.accounts.token_program.to_account_info(),
        MintTo {
          mint: _ctx.accounts.mint.to_account_info(),
          to: _ctx.accounts.mint_vault.to_account_info(),
          authority: _ctx.accounts.state.to_account_info(),
        },
        signer
      ),
      1
    )?;

    create_metadata_accounts_v3(
      CpiContext::new_with_signer(
        _ctx.accounts.metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
          metadata: _ctx.accounts.mint_metadata.to_account_info(),
          mint: _ctx.accounts.mint.to_account_info(),
          mint_authority: _ctx.accounts.state.to_account_info(),
          update_authority: _ctx.accounts.state.to_account_info(),
          payer: _ctx.accounts.authority.to_account_info(),
          system_program: _ctx.accounts.system_program.to_account_info(),
          rent: _ctx.accounts.rent.to_account_info(),
        },
        signer
      ),
      DataV2 {
        name: _name,
        symbol: _symbol,
        uri: _uri,
        seller_fee_basis_points: 100, // 1%
        creators: Some(vec![
          anchor_spl::metadata::mpl_token_metadata::types::Creator {
              address: _ctx.accounts.authority.key(),
              verified: false,
              share: 100,
          }
        ]),
        collection: None,
        uses: None,
      },
      true,
      true,
      None
    )?;

    // create master edition account
    create_master_edition_v3(
      CpiContext::new_with_signer(
        _ctx.accounts.metadata_program.to_account_info(),
        CreateMasterEditionV3 {
          edition: _ctx.accounts.mint_master.to_account_info(),
          mint: _ctx.accounts.mint.to_account_info(),
          update_authority: _ctx.accounts.state.to_account_info(),
          mint_authority: _ctx.accounts.state.to_account_info(),
          payer: _ctx.accounts.authority.to_account_info(),
          metadata: _ctx.accounts.mint_metadata.to_account_info(),
          token_program: _ctx.accounts.token_program.to_account_info(),
          system_program: _ctx.accounts.system_program.to_account_info(),
          rent: _ctx.accounts.rent.to_account_info(),
        },
        signer
      ),
      Some(_total_supply)
    )?;

    sign_metadata(
      CpiContext::new(
        _ctx.accounts.metadata_program.to_account_info(),
        SignMetadata {
          metadata: _ctx.accounts.mint_metadata.to_account_info(),
          creator: _ctx.accounts.authority.to_account_info(),
        }
      )
    )?;

    Ok(())
  }


  pub fn set_mvb_config(
    _ctx: Context<SetMvbConfig>,
    _symbol: String,
    _airdrop_root: Option<[u8; 32]>,
    _num_honey_drop_per_cycle: Option<u64>
  ) -> Result<()> {
    let mvb_account = &mut _ctx.accounts.mvb_account.load_mut()?;

    if _airdrop_root.is_some() {
      mvb_account.airdrop_root = _airdrop_root.unwrap();
    }
    if _num_honey_drop_per_cycle.is_some() {
      mvb_account.num_honey_drop_per_cycle = _num_honey_drop_per_cycle.unwrap();
    }

    Ok(())
  }

  pub fn mint_nft_airdrop(
    _ctx: Context<MintNft>,
    _symbol: String,
    _airdrop_amount: u64,
    _proof: Vec<[u8; 32]>
  ) -> Result<()> {
    let mvb_account = _ctx.accounts.mvb_account.load()?;
    let mut mvb_user_account = _ctx.accounts.mvb_user_account.as_mut().unwrap().load_mut()?;

    if mvb_user_account.airdrop_num_minted >= _airdrop_amount {
      return Err(AppErrorCode::ExceededUserAirdrop.into());
    }

    mvb_user_account.airdrop_num_minted += 1;

    let node = anchor_lang::solana_program::keccak::hashv(
      &[&_ctx.accounts.authority.key().to_bytes(), &_airdrop_amount.to_le_bytes()]
    );
    require!(merkle_proof::verify(_proof, mvb_account.airdrop_root, node.0), AppErrorCode::InvalidMerkleProof);

    drop(mvb_account);
    drop(mvb_user_account);

    return mint_nft(_ctx);
  }

  pub fn mint_nft_from_staked_lp(_ctx: Context<MintNft>, _symbol: String) -> Result<()> {
    let mut user_account = _ctx.accounts.user_account.as_mut().unwrap().load_mut()?;
    let state = _ctx.accounts.state.load()?;
    let mvb_account = _ctx.accounts.mvb_account.load()?;

    if user_account.num_minted >= state.user_max_mint {
      return Err(AppErrorCode::ExceededUserMaxMint.into());
    }
    if user_account.lp_staked_value < mvb_account.lp_value_price {
      return Err(AppErrorCode::Insufficient.into());
    }

    user_account.num_minted += 1;
    user_account.lp_staked_value -= mvb_account.lp_value_price;

    drop(user_account);
    drop(state);
    drop(mvb_account);

    return mint_nft(_ctx);
  }

  pub fn add_to_collection(_ctx: Context<AddEditionToCollection>) -> Result<()> {
    let mut mvb_edition = _ctx.accounts.mvb_edition.load_mut()?;
    mvb_edition.added_to_collection = 1;

    let seeds = &[STATE.as_bytes(), &[_ctx.bumps.state]];
    let signer = &[&seeds[..]];

    set_and_verify_sized_collection_item(
      CpiContext::new_with_signer(
        _ctx.accounts.metadata_program.to_account_info(),
        SetAndVerifySizedCollectionItem {
          collection_mint: _ctx.accounts.collection_mint.to_account_info(),
          collection_authority: _ctx.accounts.state.to_account_info(),
          collection_master_edition: _ctx.accounts.collection_master.to_account_info(),
          collection_metadata: _ctx.accounts.collection_metadata.to_account_info(),
          metadata: _ctx.accounts.new_mint_metadata.to_account_info(),
          update_authority: _ctx.accounts.state.to_account_info(),
          payer: _ctx.accounts.authority.to_account_info(),
        },
        signer
      ),
      None
    )?;

    Ok(())
  }

  pub fn create_collection(ctx: Context<CreateCollection>, _name: String, _symbol: String, _uri: String) -> Result<()> {
    let state = ctx.accounts.state.load()?;

    let seeds = &[STATE.as_bytes(), &[ctx.bumps.state]];
    let signer = &[&seeds[..]];

    let cpi_context = CpiContext::new_with_signer(
      ctx.accounts.token_program.to_account_info(),
      MintTo {
        mint: ctx.accounts.collection_mint.to_account_info(),
        to: ctx.accounts.collection_vault.to_account_info(),
        authority: ctx.accounts.state.to_account_info(),
      },
      signer
    );

    mint_to(cpi_context, 1)?;

    create_metadata_accounts_v3(
      CpiContext::new_with_signer(
        ctx.accounts.metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
          metadata: ctx.accounts.collection_metadata.to_account_info(),
          mint: ctx.accounts.collection_mint.to_account_info(),
          mint_authority: ctx.accounts.state.to_account_info(),
          update_authority: ctx.accounts.state.to_account_info(),
          payer: ctx.accounts.authority.to_account_info(),
          system_program: ctx.accounts.system_program.to_account_info(),
          rent: ctx.accounts.rent.to_account_info(),
        },
        signer
      ),
      DataV2 {
        name: _name,
        symbol: _symbol,
        uri: _uri,
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
      },
      true,
      true,
      Some(anchor_spl::metadata::mpl_token_metadata::types::CollectionDetails::V1 {
        size: state.total_supply,
      })
    )?;

    // create master edition account
    create_master_edition_v3(
      CpiContext::new_with_signer(
        ctx.accounts.metadata_program.to_account_info(),
        CreateMasterEditionV3 {
          edition: ctx.accounts.collection_master.to_account_info(),
          mint: ctx.accounts.collection_mint.to_account_info(),
          update_authority: ctx.accounts.state.to_account_info(),
          mint_authority: ctx.accounts.state.to_account_info(),
          payer: ctx.accounts.authority.to_account_info(),
          metadata: ctx.accounts.collection_metadata.to_account_info(),
          token_program: ctx.accounts.token_program.to_account_info(),
          system_program: ctx.accounts.system_program.to_account_info(),
          rent: ctx.accounts.rent.to_account_info(),
        },
        signer
      ),
      Some(0)
    )?;

    Ok(())
  }

  pub fn stake_lp(_ctx: Context<StakeLp>, _amount: u64) -> Result<()> {
    let mut user_account = _ctx.accounts.user_account.load_mut()?;

    let lp_supply = _ctx.accounts.lp_mint.supply;
    let lp_value = _ctx.accounts.lp_usd_vault.amount * 2;

    let value: u64 = u128
      ::from(_amount)
      .checked_mul(u128::from(lp_value))
      .unwrap()
      .checked_div(u128::from(lp_supply))
      .unwrap()
      .try_into()
      .unwrap();

    user_account.lp_staked_amount += _amount;
    user_account.lp_staked_value += value;
    user_account.lp_staked_time = _ctx.accounts.clock.unix_timestamp;

    let cpi_ctx = CpiContext::new(_ctx.accounts.token_program.to_account_info(), Transfer {
      from: _ctx.accounts.user_lp_vault.to_account_info(),
      to: _ctx.accounts.lp_vault.to_account_info(),
      authority: _ctx.accounts.authority.to_account_info(),
    });
    transfer(cpi_ctx, _amount)?;

    Ok(())
  }

  pub fn unstake_lp(_ctx: Context<StakeLp>) -> Result<()> {
    let mut user_account = _ctx.accounts.user_account.load_mut()?;
    let state = _ctx.accounts.state.load()?;

    if user_account.lp_staked_time + state.lock_duration > _ctx.accounts.clock.unix_timestamp {
      return Err(AppErrorCode::UnderLock.into());
    }

    let amount = user_account.lp_staked_amount;
    user_account.lp_staked_amount = 0;

    let seeds = &[STATE.as_bytes(), &[_ctx.bumps.state]];
    let signer = &[&seeds[..]];

    let cpi_ctx = CpiContext::new_with_signer(
      _ctx.accounts.token_program.to_account_info(),
      Transfer {
        from: _ctx.accounts.lp_vault.to_account_info(),
        to: _ctx.accounts.user_lp_vault.to_account_info(),
        authority: _ctx.accounts.state.to_account_info(),
      },
      signer
    );
    transfer(cpi_ctx, amount)?;

    Ok(())
  }

  pub fn lock_nft(_ctx: Context<LockNft>, _symbol: String) -> Result<()> {
    let mut lock_account = match _ctx.accounts.lock_account.load_mut() {
      Ok(r) => r,
      Err(_err) => _ctx.accounts.lock_account.load_init()?,
    };

    let edition = Edition::try_from(&_ctx.accounts.mint_edition)?;
    if edition.parent != _ctx.accounts.parent_mint_master.key() {
      return Err(AppErrorCode::InvalidEdition.into());
    }

    lock_account.authority = _ctx.accounts.authority.key();
    lock_account.parent_mint = _ctx.accounts.parent_mint.key();
    lock_account.mint = _ctx.accounts.mint.key();
    lock_account.edition = edition.edition;
    lock_account.locked_at = _ctx.accounts.clock.unix_timestamp;
    lock_account.last_calculated_at = lock_account.locked_at;
    lock_account.locking = 1;

    approve(
      CpiContext::new(_ctx.accounts.token_program.to_account_info(), anchor_spl::token::Approve {
        to: _ctx.accounts.mint_vault.to_account_info(),
        delegate: _ctx.accounts.state.to_account_info(),
        authority: _ctx.accounts.authority.to_account_info(),
      }),
      1
    )?;

    let seeds = &[STATE.as_bytes(), &[_ctx.bumps.state]];
    let signer = &[&seeds[..]];

    let cpi_ctx = CpiContext::new_with_signer(
      _ctx.accounts.metadata_program.to_account_info(),
      FreezeDelegatedAccount {
        delegate: _ctx.accounts.state.to_account_info(),
        token_account: _ctx.accounts.mint_vault.to_account_info(),
        edition: _ctx.accounts.mint_edition.to_account_info(),
        metadata: _ctx.accounts.mint_metadata.to_account_info(),
        mint: _ctx.accounts.mint.to_account_info(),
        token_program: _ctx.accounts.token_program.to_account_info(),
      },
      signer
    );
    freeze_delegated_account(cpi_ctx)?;

    Ok(())
  }

  pub fn unlock_nft(_ctx: Context<LockNft>, _symbol: String) -> Result<()> {
    let mut lock_account = _ctx.accounts.lock_account.load_mut()?;

    lock_account.calculate_lock_duration(&_ctx.accounts.clock);
    lock_account.locking = 0;

    let seeds = &[STATE.as_bytes(), &[_ctx.bumps.state]];
    let signer = &[&seeds[..]];

    let cpi_ctx = CpiContext::new_with_signer(
      _ctx.accounts.metadata_program.to_account_info(),
      ThawDelegatedAccount {
        delegate: _ctx.accounts.state.to_account_info(),
        token_account: _ctx.accounts.mint_vault.to_account_info(),
        edition: _ctx.accounts.mint_edition.to_account_info(),
        metadata: _ctx.accounts.mint_metadata.to_account_info(),
        mint: _ctx.accounts.mint.to_account_info(),
        token_program: _ctx.accounts.token_program.to_account_info(),
      },
      signer
    );
    thaw_delegated_account(cpi_ctx)?;

    revoke(
      CpiContext::new(_ctx.accounts.token_program.to_account_info(), anchor_spl::token::Revoke {
        source: _ctx.accounts.mint_vault.to_account_info(),
        authority: _ctx.accounts.authority.to_account_info(),
      })
    )?;

    Ok(())
  }

  pub fn harvest(_ctx: Context<Harvest>, _symbol: String) -> Result<()> {
    let mvb_account = _ctx.accounts.mvb_account.load()?;
    let mut lock_account = _ctx.accounts.lock_account.load_mut()?;
    let mut state = _ctx.accounts.state.load_mut()?;
    let mut user_account = _ctx.accounts.user_account.load_mut()?;

    lock_account.calculate_lock_duration(&_ctx.accounts.clock);

    if lock_account.locked_duration < state.honey_drop_cycle {
      return Err(AppErrorCode::InsufficientCycleTime.into());
    }

    lock_account.locked_duration -= state.honey_drop_cycle;

    let slothashes = &_ctx.accounts.slothashes;
    let recent_hash_data = slothashes.data.borrow();
    let most_recent = array_ref![recent_hash_data, 8, 8];

    let mut hasher = DefaultHasher::new();
    hasher.write(most_recent);
    hasher.write_u64(_ctx.accounts.clock.slot);
    hasher.write_i64(_ctx.accounts.clock.unix_timestamp);
    hasher.write(&_ctx.accounts.user_account.key().to_bytes());
    hasher.write(&_ctx.accounts.mint.key().to_bytes());

    let drops = state.take_honey_drops(&mut hasher, mvb_account.num_honey_drop_per_cycle)?;
    user_account.honey_drops
      .iter_mut()
      .zip(drops.iter())
      .for_each(|(h, d)| {
        *h += d;
      });

    Ok(())
  }

  pub fn cleanup(_ctx: Context<Cleanup>) -> Result<()> {
    Ok(())
  }
}

pub fn mint_nft(_ctx: Context<MintNft>) -> Result<()> {
  let mvb_edition = &mut _ctx.accounts.mvb_edition.load_init()?;
  let mut state = _ctx.accounts.state.load_mut()?;
  let mut mvb_account = _ctx.accounts.mvb_account.load_mut()?;

  if _ctx.accounts.clock.unix_timestamp > state.mint_end_time {
    return Err(AppErrorCode::MintEnded.into());
  }

  if mvb_account.num_minted >= mvb_account.total_supply || state.num_minted >= state.total_supply {
    return Err(AppErrorCode::ExceededTotalSupply.into());
  }

  state.num_minted += 1;
  mvb_account.num_minted += 1;

  let _edition = mvb_account.num_minted;
  mvb_edition.mint = _ctx.accounts.new_mint.key();
  mvb_edition.mvb = _ctx.accounts.mvb_account.key();
  mvb_edition.index = _edition;

  drop(state);

  let seeds = &[STATE.as_bytes(), &[_ctx.bumps.state]];
  let signer = &[&seeds[..]];

  mint_to(
    CpiContext::new_with_signer(
      _ctx.accounts.token_program.to_account_info(),
      MintTo {
        mint: _ctx.accounts.new_mint.to_account_info(),
        to: _ctx.accounts.new_mint_vault.to_account_info(),
        authority: _ctx.accounts.state.to_account_info(),
      },
      signer
    ),
    1
  )?;

  mint_new_edition_from_master_edition_via_token(
    CpiContext::new_with_signer(
      _ctx.accounts.metadata_program.to_account_info(),
      MintNewEditionFromMasterEditionViaToken {
        new_mint: _ctx.accounts.new_mint.to_account_info(),
        new_mint_authority: _ctx.accounts.state.to_account_info(),
        new_edition: _ctx.accounts.new_mint_edition.to_account_info(),
        new_metadata: _ctx.accounts.new_mint_metadata.to_account_info(),
        new_metadata_update_authority: _ctx.accounts.state.to_account_info(),

        edition_mark_pda: _ctx.accounts.new_mint_edition_mark.to_account_info(),
        master_edition: _ctx.accounts.mint_master.to_account_info(),
        metadata: _ctx.accounts.mint_metadata.to_account_info(),
        metadata_mint: _ctx.accounts.mint.to_account_info(),
        token_account: _ctx.accounts.mint_vault.to_account_info(),
        token_account_owner: _ctx.accounts.state.to_account_info(),

        payer: _ctx.accounts.authority.to_account_info(),
        token_program: _ctx.accounts.token_program.to_account_info(),
        system_program: _ctx.accounts.system_program.to_account_info(),
        rent: _ctx.accounts.rent.to_account_info(),
      },
      signer
    ),
    _edition
  )?;

  Ok(())
}

#[derive(Accounts)]
pub struct CreateState<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  /// CHECK: service account
  #[account()]
  pub service: AccountInfo<'info>,
  #[account(init, seeds = [STATE.as_bytes()], bump, payer = authority, space = 8 + size_of::<StateAccount>())]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(init, payer = authority, associated_token::mint = lp_mint, associated_token::authority = state)]
  pub lp_vault: Account<'info, TokenAccount>,
  pub lp_mint: Account<'info, Mint>,
  pub usd_mint: Account<'info, Mint>,
  pub wag_mint: Account<'info, Mint>,
  pub lp_usd_vault: Account<'info, TokenAccount>,
  pub lp_wag_vault: Account<'info, TokenAccount>,

  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetState<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(mut, seeds = [STATE.as_bytes()], bump, has_one = authority)]
  pub state: AccountLoader<'info, StateAccount>,
}

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct SetMvbConfig<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(seeds = [STATE.as_bytes()], bump, has_one = authority)]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(mut, seeds = [MVB.as_ref(), symbol.as_bytes()], bump)]
  pub mvb_account: AccountLoader<'info, MvbAccount>,
  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String, symbol: String)]
pub struct MintMasterNft<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(mut, seeds=[STATE.as_bytes()], bump, has_one = authority)]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(
    init,
    seeds = [MVB.as_ref(), symbol.as_bytes()],
    bump,
    payer = authority,
    space = 8 + size_of::<MvbAccount>()
  )]
  pub mvb_account: AccountLoader<'info, MvbAccount>,
  #[account(
    init,
    seeds = [symbol.as_bytes()],
    bump,
    payer = authority,
    mint::decimals = 0,
    mint::authority = state,
    mint::freeze_authority = state
  )]
  pub mint: Account<'info, Mint>,
  #[account(init_if_needed, payer = authority, associated_token::mint = mint, associated_token::authority = state)]
  pub mint_vault: Account<'info, TokenAccount>,
  /// CHECK: mint metadata
  #[account(mut, address = find_metadata_account(&mint.key()).0)]
  pub mint_metadata: AccountInfo<'info>,
  /// CHECK: mint master edition
  #[account(mut, address = find_master_edition_account(&mint.key()).0)]
  pub mint_master: AccountInfo<'info>,

  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub metadata_program: Program<'info, Metadata>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitUser<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(init, payer = authority, seeds = [authority.key().as_ref()], bump, space = 8 + size_of::<UserAccount>())]
  pub user_account: AccountLoader<'info, UserAccount>,

  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct InitMvbUser<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,

  #[account(seeds = [MVB.as_ref(), symbol.as_bytes()], bump)]
  pub mvb_account: AccountLoader<'info, MvbAccount>,
  #[account(
    init,
    payer = authority,
    seeds = [MVB.as_bytes(), symbol.as_bytes(), authority.key().as_ref()],
    bump,
    space = 8 + size_of::<MvbUserAccount>()
  )]
  pub mvb_user_account: AccountLoader<'info, MvbUserAccount>,

  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct MintNft<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(mut, seeds=[STATE.as_bytes()], bump)]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(mut, seeds = [MVB.as_ref(), symbol.as_bytes()],bump)]
  pub mvb_account: AccountLoader<'info, MvbAccount>,
  #[account(mut, seeds = [MVB.as_bytes(), symbol.as_bytes(), authority.key().as_ref()], bump)]
  pub mvb_user_account: Option<AccountLoader<'info, MvbUserAccount>>,
  #[account(mut, seeds = [authority.key().as_ref()], bump)]
  pub user_account: Option<AccountLoader<'info, UserAccount>>,

  #[account(seeds = [symbol.as_bytes()], bump)]
  pub mint: Box<Account<'info, Mint>>,
  #[account(mut)]
  pub mint_vault: Box<Account<'info, TokenAccount>>,
  /// CHECK: mint metadata
  #[account(mut, address = find_metadata_account(&mint.key()).0)]
  pub mint_metadata: AccountInfo<'info>,
  /// CHECK: mint master edition
  #[account(mut, address = find_master_edition_account(&mint.key()).0)]
  pub mint_master: AccountInfo<'info>,
  #[account(init, payer = authority, mint::decimals = 0, mint::authority = state, mint::freeze_authority = state)]
  pub new_mint: Box<Account<'info, Mint>>,
  #[account(
    init_if_needed,
    payer = authority,
    associated_token::mint = new_mint,
    associated_token::authority = authority
  )]
  pub new_mint_vault: Box<Account<'info, TokenAccount>>,
  /// CHECK: mint metadata
  #[account(mut, address = find_metadata_account(&new_mint.key()).0)]
  pub new_mint_metadata: AccountInfo<'info>,
  /// CHECK: mint edition
  #[account(mut, address = find_master_edition_account(&new_mint.key()).0)]
  pub new_mint_edition: AccountInfo<'info>,
  /// CHECK: mint mark
  #[account(mut, address = find_edition_account(&mint.key(), state.load()?.num_minted + 1).0)]
  pub new_mint_edition_mark: AccountInfo<'info>,

  #[account(
    init,
    payer = authority,
    space = 8 + size_of::<MvbEditionAccount>(),
    seeds = [new_mint.key().as_ref()],
    bump
  )]
  pub mvb_edition: AccountLoader<'info, MvbEditionAccount>,
  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub metadata_program: Program<'info, Metadata>,
  pub system_program: Program<'info, System>,
  pub clock: Sysvar<'info, Clock>,
  pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction()]
pub struct AddEditionToCollection<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(mut, seeds=[STATE.as_bytes()], bump)]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(mut)]
  pub new_mint: Account<'info, Mint>,
  /// CHECK: new mint metadata
  #[account(mut, address = find_metadata_account(&new_mint.key()).0)]
  pub new_mint_metadata: AccountInfo<'info>,
  #[account(mut, seeds = [new_mint.key().as_ref()], bump, )]
  pub mvb_edition: AccountLoader<'info, MvbEditionAccount>,

  #[account(mut,seeds = [COLLECTION.as_ref()], bump)]
  pub collection_mint: Box<Account<'info, Mint>>,
  /// CHECK: collection metadata
  #[account(mut, address = find_metadata_account(&collection_mint.key()).0)]
  pub collection_metadata: AccountInfo<'info>,
  /// CHECK: collection metadata
  #[account(mut, address = find_master_edition_account(&collection_mint.key()).0)]
  pub collection_master: AccountInfo<'info>,

  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub metadata_program: Program<'info, Metadata>,
  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Cleanup<'info> {
  #[account(mut)]
  pub service: Signer<'info>,
  #[account(mut, seeds=[STATE.as_bytes()], bump, has_one = service)]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(mut, close = close_authority)]
  pub mvb_edition_account: AccountLoader<'info, MvbEditionAccount>,
  /// CHECK: close authority
  #[account(mut)]
  pub close_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateCollection<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(mut, seeds=[STATE.as_bytes()], bump, has_one = authority)]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(
    init,
    seeds = [COLLECTION.as_ref()],
    bump,
    payer = authority,
    mint::decimals = 0,
    mint::authority = state,
    mint::freeze_authority = state
  )]
  pub collection_mint: Account<'info, Mint>,
  #[account(
    init_if_needed,
    payer = authority,
    associated_token::mint = collection_mint,
    associated_token::authority = state
  )]
  pub collection_vault: Account<'info, TokenAccount>,
  /// CHECK: collection metadata
  #[account(mut, address= find_metadata_account(&collection_mint.key()).0)]
  pub collection_metadata: AccountInfo<'info>,
  /// CHECK: collection metadata
  #[account(mut, address = find_master_edition_account(&collection_mint.key()).0)]
  pub collection_master: AccountInfo<'info>,

  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub metadata_program: Program<'info, Metadata>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct StakeLp<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(mut, 
        seeds=[STATE.as_bytes()], 
        bump,
        has_one = lp_mint,
        has_one = lp_vault,
        has_one = usd_mint,
        has_one = wag_mint,
        has_one = lp_usd_vault,
        has_one = lp_wag_vault,
    )]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(mut, seeds = [authority.key().as_ref()], bump)]
  pub user_account: AccountLoader<'info, UserAccount>,
  #[account(
        mut, 
        associated_token::mint = lp_mint,
        associated_token::authority = authority,
    )]
  pub user_lp_vault: Account<'info, TokenAccount>,

  pub lp_mint: Account<'info, Mint>,
  #[account(mut)]
  pub lp_vault: Account<'info, TokenAccount>,
  #[account(mut)]
  pub usd_mint: Account<'info, Mint>,
  #[account(mut)]
  pub wag_mint: Account<'info, Mint>,

  #[account()]
  pub lp_usd_vault: Account<'info, TokenAccount>,
  #[account()]
  pub lp_wag_vault: Account<'info, TokenAccount>,

  pub clock: Sysvar<'info, Clock>,
  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct LockNft<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(mut, seeds=[STATE.as_bytes()], bump)]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(
    init_if_needed,
    payer = authority,
    seeds = [LOCK.as_bytes(), authority.key().as_ref(), mint.key().as_ref()],
    bump,
    space = 8 + size_of::<NftLockAccount>()
  )]
  pub lock_account: AccountLoader<'info, NftLockAccount>,

  #[account(seeds = [symbol.as_bytes()], bump)]
  pub parent_mint: Box<Account<'info, Mint>>,
  /// CHECK: parent mint master edition
  #[account(address = find_master_edition_account(&parent_mint.key()).0)]
  pub parent_mint_master: AccountInfo<'info>,
  #[account(mut, seeds = [COLLECTION.as_ref()], bump)]
  pub collection_mint: Box<Account<'info, Mint>>,

  #[account(mut)]
  pub mint: Box<Account<'info, Mint>>,
  #[account(has_one = mint, constraint = check_metadata_collection(&mint_metadata, collection_mint.key()))]
  pub mint_metadata: Account<'info, MetadataAccount>,
  /// CHECK: mint master edition
  #[account(mut, address = find_master_edition_account(&mint.key()).0)]
  pub mint_edition: AccountInfo<'info>,
  #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = authority,
    )]
  pub mint_vault: Box<Account<'info, TokenAccount>>,

  pub clock: Sysvar<'info, Clock>,
  pub metadata_program: Program<'info, Metadata>,
  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct Harvest<'info> {
  #[account(mut)]
  pub authority: Signer<'info>,
  #[account(mut, seeds=[STATE.as_bytes()], bump)]
  pub state: AccountLoader<'info, StateAccount>,
  #[account(mut, seeds = [MVB.as_ref(), symbol.as_bytes()], bump)]
  pub mvb_account: AccountLoader<'info, MvbAccount>,

  #[account(mut, seeds = [authority.key().as_ref()], bump)]
  pub user_account: AccountLoader<'info, UserAccount>,
  #[account(
        mut,
        seeds = [LOCK.as_bytes(), authority.key().as_ref(), mint.key().as_ref()],
        bump,
        has_one = authority,
        constraint = mvb_account.load()?.mint.key() == lock_account.load()?.parent_mint.key()
    )]
  pub lock_account: AccountLoader<'info, NftLockAccount>,

  #[account(mut)]
  pub mint: Box<Account<'info, Mint>>,

  // pub recent_slothashes: Sysvar<'info, SlotHashes>,
  /// CHECK: slot hashes
  #[account(address = slot_hashes::SlotHashes::id())]
  pub slothashes: AccountInfo<'info>,
  pub clock: Sysvar<'info, Clock>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

fn check_metadata_collection(reward: &MetadataAccount, collection_mint: Pubkey) -> bool {
  match &reward.collection {
    Some(collection) => collection.key == collection_mint,
    None => false,
  }
}

#[account(zero_copy)]
pub struct StateAccount {
  pub authority: Pubkey,
  pub service: Pubkey,

  pub lp_mint: Pubkey,
  pub lp_vault: Pubkey,

  pub wag_mint: Pubkey,
  pub lp_wag_vault: Pubkey,
  pub usd_mint: Pubkey,
  pub lp_usd_vault: Pubkey,

  pub total_supply: u64,
  pub num_minted: u64,
  pub user_max_mint: u64,
  pub lock_duration: i64,
  pub mint_end_time: i64,
  pub harvest_drop_end_time: i64,
  pub honey_drop_cycle: i64,
  pub honey_drop_remains: [u64; 4],
  pub honey_drop_values: [u64; 4],
}

#[account(zero_copy)]
pub struct MvbAccount {
  pub mint: Pubkey,
  /// The 256-bit merkle root.
  pub airdrop_root: [u8; 32],
  pub total_supply: u64,
  pub num_minted: u64,
  pub lp_value_price: u64,
  pub num_honey_drop_per_cycle: u64,
}

#[account(zero_copy)]
pub struct MvbUserAccount {
  pub authority: Pubkey,
  pub mvb: Pubkey,
  pub airdrop_num_minted: u64,
  pub num_minted: u64,
}

#[account(zero_copy)]
pub struct UserAccount {
  pub authority: Pubkey,
  pub num_minted: u64,
  pub lp_staked_amount: u64,
  pub lp_staked_value: u64,
  pub lp_staked_time: i64,
  pub honey_drops: [u64; 4],
}

#[account(zero_copy)]
pub struct MvbEditionAccount {
  pub mint: Pubkey,
  pub metadata: Pubkey,
  pub mvb: Pubkey,
  pub index: u64,
  pub added_to_collection: u64,
  pub data: [Pubkey; 15],
}

#[account(zero_copy)]
pub struct NftLockAccount {
  pub parent_mint: Pubkey,
  pub authority: Pubkey,
  pub mint: Pubkey,
  pub edition: u64,
  pub locked_at: i64,
  pub last_calculated_at: i64,
  pub locked_duration: i64,
  pub honey_drops: [u64; 4],
  pub locking: u64,
}

impl NftLockAccount {
  fn calculate_lock_duration(&mut self, clock: &Clock) {
    if self.locking == 0 {
      return;
    }
    self.locked_duration += clock.unix_timestamp.checked_sub(self.last_calculated_at).unwrap();
    self.last_calculated_at = clock.unix_timestamp;
  }
}

impl StateAccount {
  fn take_honey_drops(&mut self, hasher: &mut DefaultHasher, num: u64) -> Result<[u64; 4]> {
    let mut total_drops = self.honey_drop_remains.iter().sum::<u64>();
    let mut honey_drop_clones = self.honey_drop_remains.clone();

    if num > total_drops {
      return Err(AppErrorCode::InsufficientHoneyDrop.into());
    }

    let mut honey_drop_results = [0; 4];
    for _i in 0..num {
      let random = hasher.finish();

      let last_drop_index = random % total_drops;
      let mut index = 0;
      let mut drop_acc = 0;
      for drop in honey_drop_clones.iter() {
        if *drop + drop_acc > last_drop_index {
          break;
        }
        drop_acc += *drop;
        index += 1;
      }

      honey_drop_results[index] += 1;
      honey_drop_clones[index] = honey_drop_clones[index].checked_sub(1).unwrap();
      total_drops -= 1;
    }

    self.honey_drop_remains = honey_drop_clones;

    return Ok(honey_drop_results);
  }
}

#[error_code]
pub enum AppErrorCode {
  #[msg("Exceeded total supply")]
  ExceededTotalSupply,
  #[msg("Exceeded user max mint")]
  ExceededUserMaxMint,
  #[msg("Exceeded user airdrop")]
  ExceededUserAirdrop,
  #[msg("Invalid merkle proof")]
  InvalidMerkleProof,
  #[msg("Insufficient")]
  Insufficient,
  #[msg("Insufficient cycle time")]
  InsufficientCycleTime,
  #[msg("Insufficient honey drop")]
  InsufficientHoneyDrop,
  #[msg("Under lock")]
  UnderLock,
  #[msg("Invalid edition")]
  InvalidEdition,
  #[msg("Mint ended")]
  MintEnded,
  #[msg("Harvest ended")]
  HarvestEnded,
}

pub fn find_edition_account(mint: &Pubkey, edition_number: u64) -> (Pubkey, u8) {
  Pubkey::find_program_address(
    &[
      MPL_PREFIX.as_bytes(),
      MPL_TOKEN_METADATA_ID.as_ref(),
      mint.as_ref(),
      MPL_EDITION.as_bytes(),
      edition_number.checked_div(EDITION_MARKER_BIT_SIZE).unwrap().to_string().as_bytes(),
    ],
    &MPL_TOKEN_METADATA_ID
  )
}

pub fn find_master_edition_account(mint: &Pubkey) -> (Pubkey, u8) {
  Pubkey::find_program_address(
    &[MPL_PREFIX.as_bytes(), MPL_TOKEN_METADATA_ID.as_ref(), mint.as_ref(), MPL_EDITION.as_bytes()],
    &MPL_TOKEN_METADATA_ID
  )
}

pub fn find_metadata_account(mint: &Pubkey) -> (Pubkey, u8) {
  Pubkey::find_program_address(
    &[MPL_PREFIX.as_bytes(), MPL_TOKEN_METADATA_ID.as_ref(), mint.as_ref()],
    &MPL_TOKEN_METADATA_ID
  )
}
