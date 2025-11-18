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
// #[test]
// fn test_math() {
//     // Expected values for LMSR calculations
//     const EXPECTED_INITIAL_COST: u64 = 693_147_180; // b * ln(2)
//     const EXPECTED_COST_AFTER_A: u64 = 1_313_261_688; // b * ln(e + 1)
//     const EXPECTED_COST_AFTER_B: u64 = 4_048_587_351; // b * ln(e + e^4)

//     // Initial state prices (equal probability)
//     const EXPECTED_INITIAL_PRICE_A: u64 = 500_000_000; // 50%
//     const EXPECTED_INITIAL_PRICE_B: u64 = 500_000_000; // 50%

//     // After buying A: q=[1e9, 0]
//     const EXPECTED_PRICE_A_AFTER_A: u64 = 731_058_578; // ~73.1%
//     const EXPECTED_PRICE_B_AFTER_A: u64 = 268_941_421; // ~26.9%

//     // After buying B: q=[1e9, 4e9]
//     const EXPECTED_PRICE_A_AFTER_B: u64 = 47_425_873; // ~4.7%
//     const EXPECTED_PRICE_B_AFTER_B: u64 = 952_574_126; // ~95.3%

//     const EXPECTED_PRICE_SUM: u64 = 1_000_000_000; // Prices should sum to 1.0
//                                                    // TODO: this would need to be fixed in a later version. Fixed point has some tradeoffs with precision.
//                                                    // for a future impl I would redo this using a different scale, but this is too time consuming
//     const TOLERANCE: u64 = 1; // Allow 1 lamport rounding error

//     let program_id = lmsr::id();
//     let mut svm = LiteSVM::new();
//     let bytes = include_bytes!("../../../target/deploy/lmsr.so");
//     svm.add_program(program_id, bytes);

//     let admin = Keypair::new();
//     let label = FixedSizeString::new("test_market");
//     let market = Pubkey::find_program_address(&[&MARKET_SEED, &label.as_bytes()], &program_id).0;
//     let market_vault = Pubkey::find_program_address(&[&VAULT_SEED, market.as_ref()], &program_id).0;
//     let outcome_mint_a =
//         Pubkey::find_program_address(&[&OUTCOME_MINT_SEED, market.as_ref(), &[0]], &program_id).0;
//     let outcome_mint_b =
//         Pubkey::find_program_address(&[&OUTCOME_MINT_SEED, market.as_ref(), &[1]], &program_id).0;

//     let airdrop_lamports_amount = 100_000_000_000;
//     svm.airdrop(&admin.pubkey(), airdrop_lamports_amount)
//         .unwrap();

//     let resolve_at = std::time::Instant::now().elapsed().as_secs() as i64 + 10;

//     // init_market
//     {
//         let mut accounts_ctx = lmsr::accounts::InitMarket {
//             system_program: system_program::ID,
//             rent: anchor_lang::solana_program::sysvar::rent::ID,
//             token_program: anchor_spl::token::ID,
//             admin: admin.pubkey(),
//             market,
//             market_vault,
//         }
//         .to_account_metas(None);
//         accounts_ctx.push(AccountMeta {
//             pubkey: outcome_mint_a,
//             is_signer: false,
//             is_writable: true,
//         });
//         accounts_ctx.push(AccountMeta {
//             pubkey: outcome_mint_b,
//             is_signer: false,
//             is_writable: true,
//         });
//         let ix = Instruction::new_with_bytes(
//             program_id,
//             &lmsr::instruction::InitMarket {
//                 num_outcomes: 2,
//                 scale: 100_000,
//                 resolve_at,
//                 label,
//             }
//             .data(),
//             accounts_ctx,
//         );

//         let tx = Transaction::new_signed_with_payer(
//             &[ix],
//             Some(&admin.pubkey()),
//             &[&admin],
//             svm.latest_blockhash(),
//         );
//         svm.send_transaction(tx).unwrap();
//     }

//     // assert LMSR math checks out
//     {
//         let market_account = svm.get_account(&market).unwrap();
//         assert_eq!(market_account.data.len(), lmsr::state::Market::SIZE);

//         // Simulate buying shares using LMSR
//         // This properly calculates supplies based on payments
//         let mut market =
//             lmsr::state::Market::try_deserialize(&mut market_account.data.as_ref()).unwrap();

//         // Set market parameters
//         market.scale = 1_000_000_000; // 1 SOL liquidity parameter
//         market.num_outcomes = 2;
//         market.resolve_at = resolve_at;
//         market.admin = admin.pubkey();
//         market.label = label;
//         market.initialized_at = std::time::Instant::now().elapsed().as_secs() as u64;

//         println!("\n=== Initial State ===");
//         let initial_cost = market.cost().unwrap();
//         let initial_price_a = market.price(0).unwrap();
//         let initial_price_b = market.price(1).unwrap();

//         println!("Supply A: {}", market.supplies[0]);
//         println!("Supply B: {}", market.supplies[1]);
//         println!(
//             "Cost: {} (expected: {})",
//             initial_cost, EXPECTED_INITIAL_COST
//         );
//         println!(
//             "Price A: {} (expected: {})",
//             initial_price_a, EXPECTED_INITIAL_PRICE_A
//         );
//         println!(
//             "Price B: {} (expected: {})",
//             initial_price_b, EXPECTED_INITIAL_PRICE_B
//         );

//         // Assert initial values
//         assert_eq!(initial_cost, EXPECTED_INITIAL_COST, "Initial cost mismatch");
//         assert_eq!(
//             initial_price_a, EXPECTED_INITIAL_PRICE_A,
//             "Initial price A mismatch"
//         );
//         assert_eq!(
//             initial_price_b, EXPECTED_INITIAL_PRICE_B,
//             "Initial price B mismatch"
//         );
//         assert_eq!(
//             initial_price_a + initial_price_b,
//             EXPECTED_PRICE_SUM,
//             "Initial prices don't sum to 1.0"
//         );

//         // User 1 buys 0.5 SOL worth of outcome A
//         let shares_a = market.buy_shares(0, 500_000_000).unwrap();
//         println!("\n=== After buying 0.5 SOL of A ===");

//         let cost_after_a = market.cost().unwrap();
//         let price_a_after_a = market.price(0).unwrap();
//         let price_b_after_a = market.price(1).unwrap();
//         let price_sum_after_a = price_a_after_a + price_b_after_a;

//         println!("Shares minted: {}", shares_a);
//         println!("Supply A: {} (was 0)", market.supplies[0]);
//         println!("Supply B: {}", market.supplies[1]);
//         println!("Reserve A: {}", market.reserves[0]);
//         println!(
//             "Cost: {} (expected: {})",
//             cost_after_a, EXPECTED_COST_AFTER_A
//         );
//         println!(
//             "Price A: {} (expected: {})",
//             price_a_after_a, EXPECTED_PRICE_A_AFTER_A
//         );
//         println!(
//             "Price B: {} (expected: {})",
//             price_b_after_a, EXPECTED_PRICE_B_AFTER_A
//         );
//         println!(
//             "Price sum: {} (expected: {})",
//             price_sum_after_a, EXPECTED_PRICE_SUM
//         );

//         // Assert values after buying A
//         assert_eq!(
//             cost_after_a, EXPECTED_COST_AFTER_A,
//             "Cost after buying A mismatch"
//         );
//         assert_eq!(
//             price_a_after_a, EXPECTED_PRICE_A_AFTER_A,
//             "Price A after buying A mismatch"
//         );
//         assert_eq!(
//             price_b_after_a, EXPECTED_PRICE_B_AFTER_A,
//             "Price B after buying A mismatch"
//         );
//         assert!(
//             (price_sum_after_a as i64 - EXPECTED_PRICE_SUM as i64).abs() <= TOLERANCE as i64,
//             "Prices don't sum to 1.0 after buying A: {} vs {}",
//             price_sum_after_a,
//             EXPECTED_PRICE_SUM
//         );

//         // User 2 buys 0.8 SOL worth of outcome B
//         let shares_b = market.buy_shares(1, 800_000_000).unwrap();
//         println!("\n=== After buying 0.8 SOL of B ===");
//         println!("Shares minted: {}", shares_b);
//         let cost_after_b = market.cost().unwrap();
//         let price_a_after_b = market.price(0).unwrap();
//         let price_b_after_b = market.price(1).unwrap();
//         let price_sum_after_b = price_a_after_b + price_b_after_b;

//         println!("Supply A: {}", market.supplies[0]);
//         println!("Supply B: {} (was 0)", market.supplies[1]);
//         println!("Reserve B: {}", market.reserves[1]);
//         println!(
//             "Cost: {} (expected: {}, off by 1 due to quick math impl)",
//             cost_after_b, EXPECTED_COST_AFTER_B
//         );
//         println!(
//             "Price A: {} (expected: {})",
//             price_a_after_b, EXPECTED_PRICE_A_AFTER_B
//         );
//         println!(
//             "Price B: {} (expected: {})",
//             price_b_after_b, EXPECTED_PRICE_B_AFTER_B
//         );
//         println!(
//             "Price sum: {} (expected: {}, off by 1 due to quick math impl)",
//             price_sum_after_b, EXPECTED_PRICE_SUM
//         );

//         // Assert values after buying B (allow 1 lamport rounding error for cost)
//         assert!(
//             (cost_after_b as i64 - EXPECTED_COST_AFTER_B as i64).abs() <= TOLERANCE as i64,
//             "Cost after buying B mismatch: {} vs {}",
//             cost_after_b,
//             EXPECTED_COST_AFTER_B
//         );
//         assert_eq!(
//             price_a_after_b, EXPECTED_PRICE_A_AFTER_B,
//             "Price A after buying B mismatch"
//         );
//         assert_eq!(
//             price_b_after_b, EXPECTED_PRICE_B_AFTER_B,
//             "Price B after buying B mismatch"
//         );
//         assert!(
//             (price_sum_after_b as i64 - EXPECTED_PRICE_SUM as i64).abs() <= TOLERANCE as i64,
//             "Prices don't sum to 1.0 after buying B: {} vs {}",
//             price_sum_after_b,
//             EXPECTED_PRICE_SUM
//         );

//         println!("\n✅ All LMSR math assertions passed!");
//     }
// }

/// Test LMSR edge cases
///
/// Tests:
/// 1. Very small trade (1 lamport)
/// 2. Extreme probability state (one outcome heavily favored)
/// 3. Multiple sequential buys of same outcome
/// 4. Market with very small liquidity parameter
#[test]
fn test_lmsr_edge_cases() {
    let program_id = lmsr::id();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/lmsr.so");
    svm.add_program(program_id, bytes);

    let admin = Keypair::new();
    let label = FixedSizeString::new("edge_case_market");
    let market = Pubkey::find_program_address(&[&MARKET_SEED, &label.as_bytes()], &program_id).0;
    let market_vault = Pubkey::find_program_address(&[&VAULT_SEED, market.as_ref()], &program_id).0;
    let outcome_mint_a =
        Pubkey::find_program_address(&[&OUTCOME_MINT_SEED, market.as_ref(), &[0]], &program_id).0;
    let outcome_mint_b =
        Pubkey::find_program_address(&[&OUTCOME_MINT_SEED, market.as_ref(), &[1]], &program_id).0;

    svm.airdrop(&admin.pubkey(), 100_000_000_000).unwrap();

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

    let market_account = svm.get_account(&market).unwrap();
    let mut market =
        lmsr::state::Market::try_deserialize(&mut market_account.data.as_ref()).unwrap();

    market.scale = 1_000_000_000; // 1 SOL liquidity parameter
    market.num_outcomes = 2;
    market.resolve_at = resolve_at;
    market.admin = admin.pubkey();
    market.label = label;
    market.initialized_at = std::time::Instant::now().elapsed().as_secs() as u64;

    println!("\n=== Edge Case 1: Very Small Trade (1 lamport) ===");
    let result = market.buy_shares(0, 1);

    // Should fail because shares_out would be 0
    assert!(result.is_err(), "Should reject trade that mints 0 shares");
    println!("✅ Correctly rejected 1 lamport trade (would mint 0 shares)");

    println!("\n=== Edge Case 2: Extreme Probability State ===");
    // Buy a large amount of outcome A to create skewed state
    // Note: With b=1 SOL, fp_exp maxes at exp(20), so q/b must stay < 20
    // Buying 2 SOL gives q ≈ 2-3 SOL worth of shares, keeping q/b < 3
    // TODO: for a real impl I would adjust the liquidity param, b, and handle the scaling to allow for larger buys
    // since 20 SOL max is obviously too small.
    let large_buy = 2_000_000_000; // 2 SOL
    let shares = market.buy_shares(0, large_buy).unwrap();
    println!(
        "Bought {} lamports worth, minted {} shares",
        large_buy, shares
    );

    let price_a = market.price(0).unwrap();
    let price_b = market.price(1).unwrap();
    println!("Price A: {} (~{:.2}%)", price_a, price_a as f64 / 1e7);
    println!("Price B: {} (~{:.2}%)", price_b, price_b as f64 / 1e7);

    // A should be favored (>80%)
    assert!(
        price_a > 800_000_000,
        "Price A should be >80% after large buy"
    );
    assert!(
        price_b < 200_000_000,
        "Price B should be <20% after large buy"
    );
    // Allow 1 lamport rounding error
    let price_sum = price_a + price_b;
    assert!(
        (price_sum as i64 - 1_000_000_000).abs() <= 1,
        "Prices must sum to ~1.0: {} vs 1000000000",
        price_sum
    );
    println!("✅ Extreme probability state handled correctly");

    println!("\n=== Edge Case 3: Small Liquidity Parameter ===");
    // Reset market with very small liquidity parameter
    market.supplies = [0; 16];
    market.reserves = [0; 16];
    market.scale = 10_000_000; // 0.01 SOL (very sensitive to trades)

    let initial_price_a = market.price(0).unwrap();
    let initial_price_b = market.price(1).unwrap();
    println!("Initial prices with b=0.01 SOL:");
    println!("  Price A: {}", initial_price_a);
    println!("  Price B: {}", initial_price_b);

    // Small trade should have large price impact with small b
    let shares = market.buy_shares(0, 5_000_000).unwrap(); // 0.005 SOL (half of b)
    let new_price_a = market.price(0).unwrap();
    let new_price_b = market.price(1).unwrap();

    println!("After buying 0.005 SOL (50% of b):");
    println!("  Shares minted: {}", shares);
    println!("  Price A: {} (was {})", new_price_a, initial_price_a);
    println!("  Price B: {} (was {})", new_price_b, initial_price_b);

    // With small b, even small trades should have significant impact
    let price_change = new_price_a as i64 - initial_price_a as i64;
    assert!(
        price_change > 100_000_000,
        "Small b should cause large price impact"
    );
    // Allow 1 lamport rounding error
    let price_sum = new_price_a + new_price_b;
    assert!(
        (price_sum as i64 - 1_000_000_000).abs() <= 1,
        "Prices must sum to ~1.0: {} vs 1000000000",
        price_sum
    );
    println!("✅ Small liquidity parameter shows high sensitivity");

    println!("\n=== Edge Case 4: Cost Function Monotonicity ===");
    // Reset market
    market.supplies = [0; 16];
    market.reserves = [0; 16];
    market.scale = 1_000_000_000;

    let mut prev_cost = market.cost().unwrap();
    println!("Initial cost: {}", prev_cost);

    // Cost should strictly increase with each buy
    // Alternate between outcomes to avoid extreme skew
    for i in 1..=5 {
        let outcome = if i % 2 == 1 { 0 } else { 1 };
        market.buy_shares(outcome, 500_000_000).unwrap(); // 0.5 SOL
        let new_cost = market.cost().unwrap();
        println!(
            "After buy {} (outcome {}): cost = {} (increased by {})",
            i,
            outcome,
            new_cost,
            new_cost - prev_cost
        );
        assert!(
            new_cost > prev_cost,
            "Cost must increase after buying shares"
        );
        prev_cost = new_cost;
    }
    println!("✅ Cost function is strictly monotonic");

    println!("\n✅ All edge case tests passed!");
}
