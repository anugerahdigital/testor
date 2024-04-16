use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount},
};
use ore::{
    instruction::claim, state::Proof, utils::AccountDeserialize, MINT_ADDRESS as ORE_MINT_ADDRESS,
    TREASURY_ADDRESS,
};

use crate::state::{Guild, Miner, Ore};

pub fn process_claim(ctx: Context<Claim>) -> Result<()> {
    let proof = ctx.accounts.proof_account.data.borrow();
    let proof_data = Proof::try_from_bytes(&proof).unwrap();

    let claim_ix = claim(
        ctx.accounts.miner.key(),
        ctx.accounts.guild_treasury_ata.key(),
        proof_data.claimable_rewards,
    );

    let claim_accounts = [
        ctx.accounts.miner.to_account_info(),
        ctx.accounts.guild_treasury_ata.to_account_info(),
        ctx.accounts.proof_account.to_account_info(),
        ctx.accounts.ore_treasury.to_account_info(),
        ctx.accounts.ore_treasury_ata.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
    ];

    invoke_signed(
        &claim_ix,
        &claim_accounts,
        &[&[
            Miner::PREFIX,
            ctx.accounts.guild.key().as_ref(),
            &ctx.accounts.miner.id.to_le_bytes(),
            &[ctx.bumps.miner],
        ]],
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut, has_one = authority)]
    pub guild: Account<'info, Guild>,

    #[account(mut, seeds = [Miner::PREFIX, guild.key().as_ref(), &miner.id.to_le_bytes()], bump)]
    pub miner: Account<'info, Miner>,

    /// CHECK: done via cpi
    #[account(mut, seeds = [ore::PROOF, miner.key().as_ref()], seeds::program = ore::id(), bump)]
    pub proof_account: UncheckedAccount<'info>,

    /// CHECK: manual
    #[account(mut, address = TREASURY_ADDRESS)]
    pub ore_treasury: UncheckedAccount<'info>,

    #[account(address = ORE_MINT_ADDRESS)]
    pub ore_mint: InterfaceAccount<'info, Mint>,

    #[account(mut, token::mint = ore_mint, token::authority = ore_treasury)]
    pub ore_treasury_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, token::mint = ore_mint, token::authority = authority)]
    pub guild_treasury_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub ore_program: Program<'info, Ore>,

    pub token_program: Program<'info, Token>,
}
