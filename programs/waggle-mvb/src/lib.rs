use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::mpl_token_metadata::EDITION_MARKER_BIT_SIZE,
    metadata::{
        create_master_edition_v3, create_metadata_accounts_v3,
        mint_new_edition_from_master_edition_via_token,
        mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID, set_and_verify_sized_collection_item,
        CreateMasterEditionV3, CreateMetadataAccountsV3, Metadata,
        MintNewEditionFromMasterEditionViaToken, SetAndVerifySizedCollectionItem,
    },
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};
pub mod merkle_proof;

declare_id!("B9ytUmZT7f7E6cU2qubkj3xrJnunPnW3ZSEx9pjkHgyF");

pub const MPL_PREFIX: &str = "metadata";
pub const MPL_EDITION: &str = "edition";
pub const STATE: &str = "state";
pub const MVB: &str = "mvb";
pub const MINT: &str = "mint";
pub const COLLECTION: &str = "collection";

#[program]
pub mod waggle_mvb {

    use anchor_spl::metadata::mpl_token_metadata::types::DataV2;

    use super::*;

    pub fn create_state(_ctx: Context<CreateState>, _total_supply: u64, _user_max_mint: u64) -> Result<()> {
        let state = &mut _ctx.accounts.state.load_init()?;

        state.authority = _ctx.accounts.authority.key();
        state.total_supply = _total_supply;
        state.user_max_mint = _user_max_mint;

        Ok(())
    }

    pub fn set_mvb_config(_ctx: Context<SetMvbConfig>, _name: String, _airdrop_root: [u8; 32]) -> Result<()> {
        let mvb_account = &mut _ctx.accounts.mvb_account.load_mut()?;

        mvb_account.airdrop_root = _airdrop_root;

        Ok(())
    }

    pub fn mint_master_nft(
        _ctx: Context<MintMasterNft>,
        _name: String,
        _symbol: String,
        _uri: String,
        _total_supply: u64,
    ) -> Result<()> {
        let state = _ctx.accounts.state.load()?;
        let mvb_account = &mut _ctx.accounts.mvb_account.load_init()?;
        mvb_account.mint = _ctx.accounts.mint.key();
        mvb_account.total_supply = _total_supply;

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
                signer,
            ),
            1,
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
                signer,
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
            false,
            false,
            None,
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
                signer,
            ),
            Some(state.total_supply),
        )?;

        Ok(())
    }

    pub fn mint_nft_airdrop(_ctx: Context<MintNft>, _name: String, _airdrop_amount: u64, _proof: Vec<[u8; 32]>) -> Result<()> {
        let mvb_account = _ctx.accounts.mvb_account.load()?;
        let mut mvb_user_account = _ctx.accounts.mvb_user_account.load_mut()?;

        if mvb_user_account.airdrop_num_minted >= _airdrop_amount {
            return Err(AppErrorCode::ExceededUserAirdrop.into());
        }

        mvb_user_account.airdrop_num_minted += 1;
        
        let node = anchor_lang::solana_program::keccak::hashv(&[
            &_ctx.accounts.authority.key().to_bytes(),
            &_airdrop_amount.to_le_bytes(),
        ]);
        require!(merkle_proof::verify(_proof, mvb_account.airdrop_root, node.0,), AppErrorCode::InvalidMerkleProof);

        drop(mvb_account);
        drop(mvb_user_account);

        return mint_nft(_ctx, _name);
    }

    pub fn mint_nft(_ctx: Context<MintNft>, _name: String) -> Result<()> {
        let mvb_edition = &mut _ctx.accounts.mvb_edition.load_init()?;
        let mut state = _ctx.accounts.state.load_mut()?;
        let mut mvb_account = _ctx.accounts.mvb_account.load_mut()?;
        // let mut user_account = _ctx.accounts.user_account.load_mut()?;

        // if user_account.num_minted >= state.user_max_mint {
        //     return Err(AppErrorCode::ExceededUserMaxMint.into());
        // }
        if mvb_account.num_minted >= mvb_account.total_supply
            || state.num_minted >= state.total_supply
        {
            return Err(AppErrorCode::ExceededTotalSupply.into());
        }

        mvb_edition.mint = _ctx.accounts.new_mint.key();
        mvb_edition.mvb = _ctx.accounts.mvb_account.key();

        // user_account.num_minted += 1;
        state.num_minted += 1;
        mvb_account.num_minted += 1;

        let _edition = state.num_minted;
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
                signer,
            ),
            1,
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
                signer,
            ),
            _edition,
        )?;

        Ok(())
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
                signer,
            ),
            None,
        )?;

        Ok(())
    }

    pub fn create_collection(
        ctx: Context<CreateCollection>,
        _name: String,
        _symbol: String,
        _uri: String,
    ) -> Result<()> {
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
            signer,
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
                signer,
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
            false,
            false,
            Some(
                anchor_spl::metadata::mpl_token_metadata::types::CollectionDetails::V1 {
                    size: state.total_supply,
                },
            ),
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
                signer,
            ),
            Some(0),
        )?;

        Ok(())
    }

    pub fn init_user(_ctx: Context<InitUser>) -> Result<()> {
        let mut user_account = _ctx.accounts.user_account.load_init()?;
        user_account.authority = _ctx.accounts.authority.key();
        Ok(())
    }

    pub fn init_mvb_user(_ctx: Context<InitMvbUser>, _name: String) -> Result<()> {
        let mut mvb_user_account = _ctx.accounts.mvb_user_account.load_init()?;
        mvb_user_account.authority = _ctx.accounts.authority.key();
        mvb_user_account.mvb = _ctx.accounts.mvb_account.key();
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateState<'info> {
    #[account(
        init,
        seeds = [STATE.as_bytes()],
        bump,
        payer = authority,
        space = 8 + size_of::<StateAccount>()
    )]
    pub state: AccountLoader<'info, StateAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct SetMvbConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(seeds=[STATE.as_bytes()], bump, has_one = authority)]
    pub state: AccountLoader<'info, StateAccount>,
    #[account(mut, seeds = [MVB.as_ref(), name.as_bytes()], bump)]
    pub mvb_account: AccountLoader<'info, MvbAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct MintMasterNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, seeds=[STATE.as_bytes()], bump, has_one = authority)]
    pub state: AccountLoader<'info, StateAccount>,
    #[account(
        init,
        seeds = [MVB.as_ref(), name.as_bytes()],
        bump,
        payer = authority,
        space = 8 + size_of::<MvbAccount>(),
    )]
    pub mvb_account: AccountLoader<'info, MvbAccount>,
    #[account(
        init,
        seeds = [name.as_bytes()],
        bump,
        payer = authority,
        mint::decimals = 0,
        mint::authority = state,
        mint::freeze_authority = state,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = state,
    )]
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
    #[account(
        init, 
        payer = authority, 
        seeds = [authority.key().as_ref()], 
        bump, 
        space = 8 + size_of::<UserAccount>()
    )]
    pub user_account: AccountLoader<'info, UserAccount>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct InitMvbUser<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(seeds = [MVB.as_ref(), name.as_bytes()], bump)]
    pub mvb_account: AccountLoader<'info, MvbAccount>,
    #[account(init, payer = authority, seeds = [MVB.as_bytes(), name.as_bytes(), authority.key().as_ref()], bump, space = 8 + size_of::<MvbUserAccount>())]
    pub mvb_user_account: AccountLoader<'info, MvbUserAccount>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct MintNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, seeds=[STATE.as_bytes()], bump)]
    pub state: AccountLoader<'info, StateAccount>,
    #[account(mut, seeds = [MVB.as_ref(), name.as_bytes()],bump)]
    pub mvb_account: AccountLoader<'info, MvbAccount>,
    #[account(mut, seeds = [MVB.as_bytes(), name.as_bytes(), authority.key().as_ref()], bump)]
    pub mvb_user_account: AccountLoader<'info, MvbUserAccount>,
    #[account(mut, seeds = [authority.key().as_ref()], bump)]
    pub user_account: AccountLoader<'info, UserAccount>,

    #[account(seeds = [name.as_bytes()], bump)]
    pub mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub mint_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: mint metadata
    #[account(mut, address = find_metadata_account(&mint.key()).0)]
    pub mint_metadata: AccountInfo<'info>,
    /// CHECK: mint master edition
    #[account(mut, address = find_master_edition_account(&mint.key()).0)]
    pub mint_master: AccountInfo<'info>,
    #[account(
        init,
        payer = authority,
        mint::decimals = 0,
        mint::authority = state,
        mint::freeze_authority = state,
    )]
    pub new_mint: Box<Account<'info, Mint>>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = new_mint,
        associated_token::authority = authority,
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
        bump,
    )]
    pub mvb_edition: AccountLoader<'info, MvbEditionAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
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
    #[account(mut, address = find_metadata_account(&new_mint.key()).0,    )]
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
    pub rent: Sysvar<'info, Rent>,
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
        mint::freeze_authority = state,
    )]
    pub collection_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = collection_mint,
        associated_token::authority = state,
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

#[account(zero_copy)]
pub struct StateAccount {
    pub authority: Pubkey,
    pub lp_mint: Pubkey,
    pub lp_vault: Pubkey,
    pub wag_mint: Pubkey,
    pub usd_mint: Pubkey,
    pub total_supply: u64,
    pub num_minted: u64,
    pub user_max_mint: u64,
}

#[account(zero_copy)]
pub struct MvbAccount {
    pub mint: Pubkey,
    /// The 256-bit merkle root.
    pub airdrop_root: [u8; 32],
    pub total_supply: u64,
    pub num_minted: u64,
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
}

pub fn find_edition_account(mint: &Pubkey, edition_number: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            MPL_PREFIX.as_bytes(),
            MPL_TOKEN_METADATA_ID.as_ref(),
            mint.as_ref(),
            MPL_EDITION.as_bytes(),
            edition_number
                .checked_div(EDITION_MARKER_BIT_SIZE)
                .unwrap()
                .to_string()
                .as_bytes(),
        ],
        &MPL_TOKEN_METADATA_ID,
    )
}

pub fn find_master_edition_account(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            MPL_PREFIX.as_bytes(),
            MPL_TOKEN_METADATA_ID.as_ref(),
            mint.as_ref(),
            MPL_EDITION.as_bytes(),
        ],
        &MPL_TOKEN_METADATA_ID,
    )
}

pub fn find_metadata_account(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            MPL_PREFIX.as_bytes(),
            MPL_TOKEN_METADATA_ID.as_ref(),
            mint.as_ref(),
        ],
        &MPL_TOKEN_METADATA_ID,
    )
}
