use anchor_lang::AccountDeserialize;
use litesvm::LiteSVM;
use lmsr::types::FixedSizeString;
use {
    anchor_lang::{
        prelude::AccountMeta, solana_program::instruction::Instruction, system_program,
        InstructionData, ToAccountMetas,
    },
    common::constants::{MARKET_SEED, OUTCOME_MINT_SEED, VAULT_SEED},
    solana_sdk::{
        pubkey::Pubkey,
        signer::keypair::{Keypair, Signer},
        transaction::Transaction,
    },
};

/// Test LMSR math functions
///
/// Expected values (b = 1 SOL = 1e9 lamports):
/// - Initial cost (q=[0,0]): b*ln(2) = 693,147,180 lamports
/// - After buying A (q=[1e9,0]): b*ln(e+1) = 1,313,261,688 lamports  
/// - After buying B (q=[1e9,4e9]): b*ln(e+e^4) = 4,048,587,351 lamports
#[test]
fn test_math() {
    let program_id = lmsr::id();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/lmsr.so");
    svm.add_program(program_id, bytes);

    let admin = Keypair::new();
    let label = FixedSizeString::new("test_market");
    let market = Pubkey::find_program_address(&[&MARKET_SEED, &label.as_bytes()], &program_id).0;
    let market_vault = Pubkey::find_program_address(&[&VAULT_SEED, market.as_ref()], &program_id).0;
    let outcome_mint_a =
        Pubkey::find_program_address(&[&OUTCOME_MINT_SEED, market.as_ref(), &[0]], &program_id).0;
    let outcome_mint_b =
        Pubkey::find_program_address(&[&OUTCOME_MINT_SEED, market.as_ref(), &[1]], &program_id).0;

    let airdrop_lamports_amount = 100_000_000_000;
    svm.airdrop(&admin.pubkey(), airdrop_lamports_amount)
        .unwrap();

    let resolve_at = std::time::Instant::now().elapsed().as_secs() as i64 + 10;

    // init_market
    {
        let mut accounts_ctx = lmsr::accounts::InitMarket {
            system_program: system_program::ID,
            rent: anchor_lang::solana_program::sysvar::rent::ID,
            token_program: anchor_spl::token::ID,
            admin: admin.pubkey(),
            market,
            market_vault,
        }
        .to_account_metas(None);
        accounts_ctx.push(AccountMeta {
            pubkey: outcome_mint_a,
            is_signer: false,
            is_writable: true,
        });
        accounts_ctx.push(AccountMeta {
            pubkey: outcome_mint_b,
            is_signer: false,
            is_writable: true,
        });
        let ix = Instruction::new_with_bytes(
            program_id,
            &lmsr::instruction::InitMarket {
                num_outcomes: 2,
                scale: 100_000,
                resolve_at,
                label,
            }
            .data(),
            accounts_ctx,
        );

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            svm.latest_blockhash(),
        );
        svm.send_transaction(tx).unwrap();
    }

    // assert LMSR math checks out
    {
        let market_account = svm.get_account(&market).unwrap();
        assert_eq!(market_account.data.len(), lmsr::state::Market::SIZE);

        // Simulate buying shares using LMSR
        // This properly calculates supplies based on payments
        let mut market =
            lmsr::state::Market::try_deserialize(&mut market_account.data.as_ref()).unwrap();

        // Set market parameters
        market.scale = 1_000_000_000; // 1 SOL liquidity parameter
        market.num_outcomes = 2;
        market.resolve_at = resolve_at;
        market.admin = admin.pubkey();
        market.label = label;
        market.initialized_at = std::time::Instant::now().elapsed().as_secs() as u64;

        println!("\n=== Initial State (Expected: cost=693147180) ===");
        println!("Supply A: {}", market.supplies[0]);
        println!("Supply B: {}", market.supplies[1]);
        println!("Cost: {}", market.cost().unwrap());
        println!("Price A: {}", market.price(0).unwrap());
        println!("Price B: {}", market.price(1).unwrap());

        // User 1 buys 0.5 SOL worth of outcome A
        let shares_a = market.buy_shares(0, 500_000_000).unwrap();
        println!("\n=== After buying 0.5 SOL of A ===");
        println!("Shares minted: {}", shares_a);
        println!("Supply A: {} (was 0)", market.supplies[0]);
        println!("Supply B: {}", market.supplies[1]);
        println!("Reserve A: {}", market.reserves[0]);
        println!("Cost: {} (was 668771400)", market.cost().unwrap());
        println!("Price A: {}", market.price(0).unwrap());
        println!("Price B: {}", market.price(1).unwrap());

        // User 2 buys 0.8 SOL worth of outcome B
        let shares_b = market.buy_shares(1, 800_000_000).unwrap();
        println!("\n=== After buying 0.8 SOL of B ===");
        println!("Shares minted: {}", shares_b);
        println!("Supply A: {}", market.supplies[0]);
        println!("Supply B: {} (was 0)", market.supplies[1]);
        println!("Reserve B: {}", market.reserves[1]);
        println!("Cost: {} (should be > 693147180)", market.cost().unwrap());
        println!("Price A: {}", market.price(0).unwrap());
        println!("Price B: {}", market.price(1).unwrap());

        // Verify prices sum to ~1.0
        let price_a = market.price(0).unwrap();
        let price_b = market.price(1).unwrap();
        let price_sum = price_a + price_b;
        println!("\nPrice sum: {} (should be ~1_000_000_000)", price_sum);

        // Prices should sum to approximately 1e9 (allowing for rounding)
        assert!((price_sum as i64 - 1_000_000_000).abs() < 100);
    }
}
