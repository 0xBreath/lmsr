use crate::types::FixedSizeString;
use anchor_lang::prelude::*;
use common::check_condition;
use common::constants::common::*;
use common::constants::MAX_OUTCOMES;
use common::errors::ErrorCode;

#[account(zero_copy)]
#[derive(InitSpace, Default)]
#[repr(C)]
pub struct Market {
    /// Reserves for each outcome, fixed-point scaled.
    /// All values stored as u64 but promoted to u128 for math.
    pub reserves: [u64; MAX_OUTCOMES],

    /// Outcome mint token supplies for each outcome, fixed-point scaled.
    /// All values stored as u64 but promoted to u128 for math.
    /// Each outcome has a unique mint but all have the same decimals, so this is safe to apply generic math to.
    pub supplies: [u64; MAX_OUTCOMES],

    /// LMSR liquidity parameter 'b' (in lamports)
    /// Controls market depth - higher values mean more liquidity and smaller price impact
    pub scale: u64,

    pub initialized_at: u64,

    /// When the market will resolve and halt trading
    pub resolve_at: i64,

    /// The admin of the market who can mutate it
    pub admin: Pubkey,

    pub label: FixedSizeString,

    /// Number of outcomes (N)
    pub num_outcomes: u8,

    /// Bump for this [`Market`]
    pub bump: u8,

    /// Bump for market_vault which contains SOL reserves on behalf of the [`Market`]
    pub vault_bump: u8,

    /// Padding for zero copy alignment
    pub _padding: [u8; 13],
}

impl Market {
    pub const SIZE: usize = 8 + Market::INIT_SPACE;
}

/// Fixed-point exponential function: exp(x) where x is scaled by 1e9
/// Returns result scaled by 1e9
/// Uses Taylor series: exp(x) = 1 + x + x²/2! + x³/3! + ...
/// Accurate for x in range [-10, 10] (scaled)
/// NOTE: this should be linear approximation on-chain if possible, but if large trades are allowed then that is not feasible.
fn fp_exp(x: i128) -> Result<u128> {
    if x > 20 * D9_I128 {
        return Ok(u128::MAX);
    }
    if x < -20 * D9_I128 {
        return Ok(0);
    }

    // Taylor series: exp(x) = 1 + x + x²/2! + x³/3! + x⁴/4! + ...
    let mut result: i128 = D9_I128; // Start with 1.0
    let mut term: i128 = D9_I128; // Current term in series

    // 20 terms is accurate enough but arbitrary
    for n in 1..=20 {
        // term = term * x / n
        term = (term * x) / D9_I128 / (n as i128);

        if term.abs() < 1 {
            break; // Convergence reached
        }

        result = result
            .checked_add(term)
            .ok_or(error!(ErrorCode::MathOverflow))?;
    }

    if result < 0 {
        Ok(0)
    } else {
        Ok(result as u128)
    }
}

/// Fixed-point natural logarithm: ln(x) where x is scaled by 1e9
/// Returns result scaled by 1e9
/// Uses Taylor series around x=1: ln(x) = (x-1) - (x-1)²/2 + (x-1)³/3 - ...
/// NOTE: this should be linear approximation on-chain if possible, but if large trades are allowed then that is not feasible.
fn fp_ln(x: u128) -> Result<i128> {
    if x == 0 {
        return Err(error!(ErrorCode::MathOverflow)); // ln(0) is undefined
    }

    if x == D9_I128 as u128 {
        return Ok(0); // ln(1) = 0
    }

    let x_i128 = x as i128;

    // For better convergence, use ln(x) = -ln(1/x) if x < 1
    if x < D9_I128 as u128 {
        let inv = (D9_I128 * D9_I128) / x_i128;
        return fp_ln(inv as u128).map(|v| -v);
    }

    // For x > 2, use ln(x) = ln(x/e) + 1 to bring closer to 1
    // e ≈ 2.718281828, scaled = 2718281828
    const E_SCALED: i128 = 2_718_281_828;
    if x > (2 * D9_I128 as u128) {
        let reduced = (x_i128 * D9_I128) / E_SCALED;
        return fp_ln(reduced as u128).map(|v| v + D9_I128);
    }

    // Taylor series: ln(1+y) = y - y²/2 + y³/3 - y⁴/4 + ...
    // where y = x - 1
    let y = x_i128 - D9_I128;
    let mut result: i128 = 0;
    let mut y_power = y;

    // 20 terms is accurate enough but arbitrary
    for n in 1..=20 {
        let sign = if n % 2 == 1 { 1 } else { -1 };
        let term = (y_power * sign) / (n as i128);

        if term.abs() < 1 {
            break;
        }

        result = result
            .checked_add(term)
            .ok_or(error!(ErrorCode::MathOverflow))?;
        y_power = (y_power * y) / D9_I128;
    }

    Ok(result)
}

impl Market {
    /// Compute the LMSR cost function which is how much SOL (reserves) is needed to replicate the market based on parameters q and b.
    ///
    /// LMSR cost function:
    /// C(q) = b * ln(Σ exp(q_i / b))
    ///
    /// Where:
    /// - b is the liquidity parameter (self.scale which determines sensitivity to price impact; steepness of the curve)
    /// - q_i is the quantity of shares for outcome i (self.supplies[i])
    ///
    /// Returns the cost in lamports
    pub fn cost(&self) -> Result<u64> {
        let n = self.num_outcomes as usize;
        check_condition!(n <= MAX_OUTCOMES, InvalidOutcomeIndex);

        let b = self.scale as u128;
        check_condition!(b > 0, ReserveIsZero);

        const SCALE: i128 = 1_000_000_000; // 1e9 for fixed-point

        // Calculate Σ exp(q_i / b)
        // Supplies are stored scaled by 1e9, so q_i / b gives ratio scaled by 1e9
        let mut sum_exp: u128 = 0;
        for i in 0..n {
            let q_i_scaled = self.supplies[i] as i128;
            let exp_arg = q_i_scaled / (b as i128); // q_scaled / b gives ratio scaled by 1e9
            let exp_val = fp_exp(exp_arg)?;
            sum_exp = sum_exp
                .checked_add(exp_val)
                .ok_or(error!(ErrorCode::MathOverflow))?;
        }

        // Calculate C(q) = b * ln(sum)
        let ln_sum = fp_ln(sum_exp)?;
        let cost_i128 = ((b as i128) * ln_sum) / SCALE;

        // Cost should always be non-negative for valid market states
        check_condition!(cost_i128 >= 0, MathOverflow);

        Ok(cost_i128 as u64)
    }

    /// Compute how many shares to mint based on the LMSR cost function.
    /// Takes lamports in exchange.
    ///
    /// Updates:
    /// - supplies[outcome_index] increases by calculated shares (supply)
    /// - reserves[outcome_index] increases by lamports (reserves)
    ///
    /// Return the shares (supply) minted
    pub fn buy_shares(&mut self, outcome_index: usize, amount_in: u64) -> Result<u64> {
        let n = self.num_outcomes as usize;
        check_condition!(outcome_index < n, InvalidOutcomeIndex);
        check_condition!(amount_in > 0, DepositIsZero);

        let b = self.scale as u128;
        check_condition!(b > 0, LiquidityParameterIsZero);

        // Δq = b * ln(S * (exp(amount_in/b) - 1) / exp(q_i/b) + 1)

        // S = Σ exp(q_j / b)
        // Note: supplies are stored scaled by 1e9, b is in lamports
        // So (q_j / 1e9) / b gives the dimensionless ratio
        // Simplified: q_j / (b * 1e9) then scale by 1e9 for fp_exp: (q_j * 1e9) / (b * 1e9) = q_j / b
        let mut sum_exp: u128 = 0;
        for i in 0..n {
            let q_j_scaled = self.supplies[i] as i128; // Already scaled by 1e9
            let exp_arg = q_j_scaled / (b as i128); // q_scaled / b gives ratio scaled by 1e9
            let exp_val = fp_exp(exp_arg)?;
            sum_exp = sum_exp
                .checked_add(exp_val)
                .ok_or(error!(ErrorCode::MathOverflow))?;
        }

        // exp(q_i / b)
        let q_i_scaled = self.supplies[outcome_index] as i128;
        let exp_qi_b = fp_exp(q_i_scaled / (b as i128))?;

        // exp(amount_in / b)
        let amount_scaled = (amount_in as i128) * D9_I128;
        let exp_amount_b = fp_exp(amount_scaled / (b as i128))?;

        // Δq = b * ln(S * (exp(amount_in/b) - 1) / exp(q_i/b) + 1)
        let numerator = sum_exp
            .checked_mul(
                exp_amount_b
                    .checked_sub(D9_I128 as u128)
                    .ok_or(error!(ErrorCode::MathOverflow))?,
            )
            .ok_or(error!(ErrorCode::MathOverflow))?
            / (D9_I128 as u128);

        let fraction = numerator
            .checked_div(exp_qi_b)
            .ok_or(error!(ErrorCode::MathOverflow))?;
        let ln_arg = fraction
            .checked_add(D9_I128 as u128)
            .ok_or(error!(ErrorCode::MathOverflow))?;
        let ln_result = fp_ln(ln_arg)?;

        // Δq = b * ln(...)
        // b is in lamports, ln_result is scaled by 1e9
        // Result: b * ln_result is shares scaled by 1e9 (which is how we store supplies)
        let shares_out = ((b as i128) * ln_result) as u64;
        check_condition!(shares_out > 0, DepositIsZero);

        self.supplies[outcome_index] = self.supplies[outcome_index]
            .checked_add(shares_out)
            .ok_or(error!(ErrorCode::MathOverflow))?;

        self.reserves[outcome_index] = self.reserves[outcome_index]
            .checked_add(amount_in)
            .ok_or(error!(ErrorCode::MathOverflow))?;

        Ok(shares_out)
    }

    /// Compute LMSR price/probability for an outcome.
    /// Returns u64 scaled by 1e9 for safe math (i.e. 1.0 = 1_000_000_000).
    ///
    /// LMSR price formula:
    /// p_i = exp(q_i / b) / Σ exp(q_j / b)
    ///
    /// Where:
    /// - q_i is the quantity of shares for outcome i (supply)
    /// - b is the liquidity parameter
    /// - The sum is over all outcomes
    ///
    /// This gives the price/probability for each outcome.
    /// Prices always sum to exactly 1.0 (100%) across all outcomes.
    pub fn price(&self, outcome_index: usize) -> Result<u64> {
        let n = self.num_outcomes as usize;
        check_condition!(n <= MAX_OUTCOMES, InvalidOutcomeIndex);
        check_condition!(outcome_index < n, InvalidOutcomeIndex);

        let b = self.scale as u128;
        check_condition!(b > 0, LiquidityParameterIsZero);

        // Calculate exp(q_i / b) for the target outcome
        // Supplies are stored scaled by 1e9, so q_i / b gives ratio scaled by 1e9
        let q_i_scaled = self.supplies[outcome_index] as i128;
        let exp_qi_b = fp_exp(q_i_scaled / (b as i128))?;

        // Calculate Σ exp(q_j / b) for all outcomes
        let mut sum_exp: u128 = 0;
        for i in 0..n {
            let q_j_scaled = self.supplies[i] as i128;
            let exp_arg = q_j_scaled / (b as i128);
            let exp_val = fp_exp(exp_arg)?;
            sum_exp = sum_exp
                .checked_add(exp_val)
                .ok_or(error!(ErrorCode::MathOverflow))?;
        }

        // Handle edge case: if sum is zero (shouldn't happen)
        if sum_exp == 0 {
            return Ok(0);
        }

        // Compute price: (exp(q_i/b) / sum) * 1e9
        // This gives the probability/price scaled by 1e9
        let price = exp_qi_b
            .checked_mul(D9_U128)
            .ok_or(error!(ErrorCode::MathOverflow))?
            .checked_div(sum_exp)
            .ok_or(error!(ErrorCode::MathOverflow))?;

        // Clamp to u64::MAX if somehow exceeds (shouldn't happen in practice)
        if price > u64::MAX as u128 {
            Ok(u64::MAX)
        } else {
            Ok(price as u64)
        }
    }
}
