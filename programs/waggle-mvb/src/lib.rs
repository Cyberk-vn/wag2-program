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

    pub fn create_state(_ctx: Context<CreateState>, _total_supply: u64) -> Result<()> {
        let state = &mut _ctx.accounts.state.load_init()?;

        state.authority = *_ctx.accounts.authority.key;
        state.total_supply = _total_supply;

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
        let mvb_mint_account = &mut _ctx.accounts.mvb_mint_account.load_init()?;
        mvb_mint_account.mint = _ctx.accounts.mint.key();
        mvb_mint_account.total_supply = _total_supply;

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

    pub fn mint_nft(_ctx: Context<MintNft>, _name: String) -> Result<()> {
        let mvb_edition = &mut _ctx.accounts.mvb_edition.load_init()?;
        let mut state = _ctx.accounts.state.load_mut()?;
        let mut mvb_account = _ctx.accounts.mvb_mint_account.load_mut()?;

        if mvb_account.num_minted >= mvb_account.total_supply
            || state.num_minted >= state.total_supply
        {
            return Err(ErrorCode::ExceededTotalSupply.into());
        }

        mvb_edition.mint = _ctx.accounts.new_mint.key();
        mvb_edition.mvb = _ctx.accounts.mvb_mint_account.key();

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
pub struct MintMasterNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, seeds=[STATE.as_bytes()], bump, has_one = authority)]
    pub state: AccountLoader<'info, StateAccount>,
    #[account(
        init,
        seeds = [b"mvb".as_ref(), name.as_bytes()],
        bump,
        payer = authority,
        space = 8 + size_of::<MvbMintAccount>(),
    )]
    pub mvb_mint_account: AccountLoader<'info, MvbMintAccount>,
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
#[instruction(name: String)]
pub struct MintNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, seeds=[STATE.as_bytes()], bump)]
    pub state: AccountLoader<'info, StateAccount>,
    #[account(mut, seeds = [b"mvb".as_ref(), name.as_bytes()],bump)]
    pub mvb_mint_account: AccountLoader<'info, MvbMintAccount>,
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
    pub total_supply: u64,
    pub num_minted: u64,
}

#[account(zero_copy)]
pub struct MvbMintAccount {
    pub mint: Pubkey,
    pub total_supply: u64,
    pub num_minted: u64,
}

#[account(zero_copy)]
pub struct MvbEditionAccount {
    pub mint: Pubkey,             // 32
    pub metadata: Pubkey,         // 32
    pub mvb: Pubkey,              // 32
    pub index: u64,               // 8
    pub added_to_collection: u64, // 8
    pub data: [Pubkey; 15],
}

#[error_code]
pub enum ErrorCode {
    #[msg("Exceeded total supply")]
    ExceededTotalSupply,
    #[msg("Oh No")]
    OhNo,
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
