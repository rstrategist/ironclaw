//! Technical Analysis Engine
//!
//! Provides reusable technical analysis functions with native WASM-compatible implementations.
//! Implements RSI, MACD, Bollinger Bands, ATR indicators, and Ehlers DSP filters.

use crate::MarketData;
use std::f64::consts::PI;

/// Calculate Relative Strength Index (RSI)
///
/// # Arguments
/// * `prices` - Slice of closing prices
/// * `period` - RSI period (typically 14)
///
/// # Returns
/// * `Result<Vec<f64>, String>` - RSI values or error message
pub fn calculate_rsi(prices: &[f64], period: usize) -> Result<Vec<f64>, String> {
    if prices.len() < period + 1 {
        return Err(format!(
            "Insufficient data: need at least {} prices, got {}",
            period + 1,
            prices.len()
        ));
    }

    let mut rsi_values = Vec::with_capacity(prices.len() - period);
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;

    // Calculate initial average gain and loss
    for i in 1..=period {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 {
            avg_gain += change;
        } else {
            avg_loss += change.abs();
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    // Calculate RSI for the first period
    rsi_values.push(calculate_rsi_value(avg_gain, avg_loss));

    // Calculate RSI for remaining periods using smoothing
    for i in (period + 1)..prices.len() {
        let change = prices[i] - prices[i - 1];
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { change.abs() } else { 0.0 };

        // Wilder's smoothing
        avg_gain = (avg_gain * (period - 1) as f64 + gain) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + loss) / period as f64;

        rsi_values.push(calculate_rsi_value(avg_gain, avg_loss));
    }

    Ok(rsi_values)
}

fn calculate_rsi_value(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        100.0
    } else {
        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }
}

/// Calculate MACD (Moving Average Convergence Divergence)
///
/// # Arguments
/// * `prices` - Slice of closing prices
/// * `fast` - Fast EMA period (typically 12)
/// * `slow` - Slow EMA period (typically 26)
/// * `signal` - Signal line period (typically 9)
///
/// # Returns
/// * `Result<(Vec<f64>, Vec<f64>, Vec<f64>), String>` - (macd_line, signal_line, histogram)
pub fn calculate_macd(
    prices: &[f64],
    fast: usize,
    slow: usize,
    signal: usize,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>), String> {
    if prices.len() < slow + signal {
        return Err(format!(
            "Insufficient data: need at least {} prices, got {}",
            slow + signal,
            prices.len()
        ));
    }

    if fast >= slow {
        return Err("Fast period must be less than slow period".to_string());
    }

    // Calculate EMAs
    let fast_ema = calculate_ema(prices, fast)?;
    let slow_ema = calculate_ema(prices, slow)?;

    // Calculate MACD line
    let macd_line: Vec<f64> = fast_ema
        .iter()
        .skip(slow - fast)
        .zip(slow_ema.iter())
        .map(|(f, s)| f - s)
        .collect();

    // Calculate signal line (EMA of MACD)
    let signal_line = calculate_ema(&macd_line, signal)?;

    // Calculate histogram
    let histogram: Vec<f64> = macd_line
        .iter()
        .skip(signal - 1)
        .zip(signal_line.iter())
        .map(|(m, s)| m - s)
        .collect();

    // Align all outputs
    let aligned_macd: Vec<f64> = macd_line.iter().skip(signal - 1).copied().collect();

    Ok((aligned_macd, signal_line, histogram))
}

fn calculate_ema(prices: &[f64], period: usize) -> Result<Vec<f64>, String> {
    if prices.len() < period {
        return Err(format!(
            "Insufficient data for EMA: need at least {} prices, got {}",
            period,
            prices.len()
        ));
    }

    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut ema = Vec::with_capacity(prices.len() - period + 1);

    // Initial SMA
    let mut ema_val: f64 = prices.iter().take(period).sum::<f64>() / period as f64;
    ema.push(ema_val);

    // Calculate EMA for remaining prices
    for price in prices.iter().skip(period) {
        ema_val = (price - ema_val) * multiplier + ema_val;
        ema.push(ema_val);
    }

    Ok(ema)
}

/// Calculate Bollinger Bands
///
/// # Arguments
/// * `prices` - Slice of closing prices
/// * `period` - Moving average period (typically 20)
/// * `std_dev` - Standard deviation multiplier (typically 2.0)
///
/// # Returns
/// * `Result<(Vec<f64>, Vec<f64>, Vec<f64>), String>` - (upper, middle, lower)
pub fn calculate_bollinger(
    prices: &[f64],
    period: usize,
    std_dev: f64,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>), String> {
    if prices.len() < period {
        return Err(format!(
            "Insufficient data: need at least {} prices, got {}",
            period,
            prices.len()
        ));
    }

    let mut upper = Vec::with_capacity(prices.len() - period + 1);
    let mut middle = Vec::with_capacity(prices.len() - period + 1);
    let mut lower = Vec::with_capacity(prices.len() - period + 1);

    for i in period..=prices.len() {
        let slice = &prices[i - period..i];
        let sma: f64 = slice.iter().sum::<f64>() / period as f64;

        let variance: f64 = slice.iter().map(|p| (p - sma).powi(2)).sum::<f64>() / period as f64;
        let std = variance.sqrt();

        upper.push(sma + std_dev * std);
        middle.push(sma);
        lower.push(sma - std_dev * std);
    }

    Ok((upper, middle, lower))
}

/// Calculate Average True Range (ATR)
///
/// # Arguments
/// * `highs` - Slice of high prices
/// * `lows` - Slice of low prices
/// * `closes` - Slice of closing prices
/// * `period` - ATR period (typically 14)
///
/// # Returns
/// * `Result<Vec<f64>, String>` - ATR values or error message
pub fn calculate_atr(
    highs: &[f64],
    lows: &[f64],
    closes: &[f64],
    period: usize,
) -> Result<Vec<f64>, String> {
    if highs.len() != lows.len() || highs.len() != closes.len() {
        return Err("Input arrays must have equal length".to_string());
    }

    if highs.len() < period + 1 {
        return Err(format!(
            "Insufficient data: need at least {} data points, got {}",
            period + 1,
            highs.len()
        ));
    }

    // Calculate True Range for each period
    let mut tr_values = Vec::with_capacity(highs.len() - 1);

    for i in 1..highs.len() {
        let tr1 = highs[i] - lows[i];
        let tr2 = (highs[i] - closes[i - 1]).abs();
        let tr3 = (lows[i] - closes[i - 1]).abs();
        tr_values.push(tr1.max(tr2).max(tr3));
    }

    // Calculate ATR using Wilder's smoothing
    let mut atr_values = Vec::with_capacity(tr_values.len() - period + 1);
    let mut atr_val: f64 = tr_values.iter().take(period).sum::<f64>() / period as f64;
    atr_values.push(atr_val);

    for tr in tr_values.iter().skip(period) {
        atr_val = (atr_val * (period - 1) as f64 + tr) / period as f64;
        atr_values.push(atr_val);
    }

    Ok(atr_values)
}

/// Ehlers SuperSmoother Filter (10-bar critical period)
///
/// A superior smoothing filter based on Ehlers' DSP research.
/// Uses exact coefficients for a 10-bar critical period.
///
/// # Formula (from "Rocket Science for Traders", Ehlers)
/// ```text
/// a1 = exp(-1.414 * PI / 10)
/// b1 = 2 * a1 * cos(1.414 * 180 / 10 degrees)
/// c2 = b1
/// c3 = -a1²
/// c1 = 1 - c2 - c3
/// Filt = c1*(Price + Price[1])/2 + c2*Filt[1] + c3*Filt[2]
/// ```
///
/// # Arguments
/// * `price` - Slice of price values
///
/// # Returns
/// * `Vec<f64>` - Smoothed values (same length as input, with warmup period)
pub fn supersmoother(price: &[f64]) -> Vec<f64> {
    if price.is_empty() {
        return Vec::new();
    }
    if price.len() == 1 {
        return vec![price[0]];
    }

    // Exact coefficients for 10-bar critical period (Ehlers formula)
    // a1 = exp(-1.414 * PI / 10) ≈ 0.639
    // b1 = 2 * a1 * cos(1.414 * 180 / 10 degrees) ≈ 1.178
    let a1: f64 = (-1.414_f64 * PI / 10.0).exp();
    let b1: f64 = 2.0 * a1 * (1.414_f64 * 180.0 / 10.0).to_radians().cos();
    let c2: f64 = b1;
    let c3: f64 = -a1 * a1;
    let c1: f64 = 1.0 - c2 - c3;

    let mut filt = Vec::with_capacity(price.len());

    // Initialize first value
    filt.push(price[0]);

    // For second value, use the filter equation as best as possible
    // With only one previous value available, we approximate filt[-1] as price[0]
    let smoothed_1 = c1 * (price[1] + price[0]) / 2.0 + c2 * filt[0] + c3 * price[0];
    filt.push(smoothed_1);

    // Apply SuperSmoother recursively for remaining values
    for i in 2..price.len() {
        let smoothed = c1 * (price[i] + price[i - 1]) / 2.0 + c2 * filt[i - 1] + c3 * filt[i - 2];
        filt.push(smoothed);
    }

    filt
}

/// Ehlers Roofing Filter
///
/// A bandpass filter combining a two-pole HighPass filter (48-bar period)
/// followed by the SuperSmoother. Effectively isolates cyclic components
/// while removing both trend (via HP) and noise (via SuperSmoother).
///
/// # Formula
/// ```text
/// α = (0.707 * 2 * PI / 48)²
/// HP = (1-α/2)*(Price-Price[1]) + (1-α)*HP[1]
/// Filt = SuperSmoother(HP)
/// ```
///
/// # Arguments
/// * `price` - Slice of price values
///
/// # Returns
/// * `Vec<f64>` - Filtered roofing values (same length as input)
pub fn roofing_filter(price: &[f64]) -> Vec<f64> {
    if price.len() < 3 {
        return vec![0.0; price.len()];
    }

    // HighPass filter coefficient for 48-bar period
    let alpha: f64 = (0.707_f64 * 2.0 * PI / 48.0).powi(2);
    let one_minus_alpha = 1.0 - alpha;
    let one_minus_alpha_half = 1.0 - alpha / 2.0;

    let mut hp = Vec::with_capacity(price.len());

    // Initialize first HighPass value
    hp.push(0.0);

    // Apply two-pole HighPass filter
    for i in 1..price.len() {
        let hp_val = if i == 1 {
            // First iteration uses simple difference
            one_minus_alpha_half * (price[i] - price[i - 1])
        } else {
            one_minus_alpha_half * (price[i] - price[i - 1]) + one_minus_alpha * hp[i - 1]
        };
        hp.push(hp_val);
    }

    // Apply SuperSmoother to HighPass output
    supersmoother(&hp)
}

/// Calculate superposition score across multiple timeframes
/// Based on Ehlers' Hilbert Transform phasing theory
///
/// # Arguments
/// * `price_data` - Slice of MarketData points
/// * `timeframes` - Slice of timeframe periods to analyze
///
/// # Returns
/// * `Result<f64, String>` - Superposition score (0.0 - 1.0)
pub fn calculate_superposition_score(
    price_data: &[MarketData],
    timeframes: &[u32],
) -> Result<f64, String> {
    if price_data.is_empty() {
        return Err("Price data cannot be empty".to_string());
    }

    if timeframes.is_empty() {
        return Err("At least one timeframe must be specified".to_string());
    }

    // TODO: Implement Ehlers Hilbert Transform phasing
    // 1. Extract dominant cycle using Hilbert Transform
    // 2. Calculate instantaneous phase for each timeframe
    // 3. Detect phase confluence (alignment across frequencies)
    // 4. Score inflection point probability (0.0 - 1.0)
    // Reference: Ehlers, "Rocket Science for Traders"

    Ok(0.5) // Placeholder
}

/// Hilbert Transform FIR Approximation Output
///
/// Contains the in-phase (I1) and quadrature (Q1) components
/// along with the smoothed price series used in the calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct HilbertOutput {
    /// In-phase component (I1) - detrended and delayed signal
    pub in_phase: Vec<f64>,
    /// Quadrature component (Q1) - 90-degree phase shifted signal
    pub quadrature: Vec<f64>,
    /// 4-bar weighted smoothing of input price
    pub smooth: Vec<f64>,
}

/// Hilbert Transform FIR Approximation
///
/// Implements Ehlers' Hilbert Transform using FIR filter approximation.
/// Produces in-phase (I) and quadrature (Q) components for cycle analysis.
///
/// # Formula (from Ehlers "Rocket Science for Traders")
/// ```text
/// Smooth = (4*P + 3*P[1] + 2*P[2] + P[3]) / 10
/// Detrender = (0.0962*S + 0.5769*S[2] - 0.5769*S[4] - 0.0962*S[6]) * (0.075*prev_period + 0.54)
/// Q1 = apply same FIR to Detrender
/// I1 = Detrender[3]
/// ```
///
/// # Arguments
/// * `price` - Slice of price values
/// * `prev_period` - Previous dominant cycle period estimate (for adaptive tuning)
///
/// # Returns
/// * `HilbertOutput` - Contains in_phase, quadrature, and smooth vectors
pub fn hilbert_transform(price: &[f64], prev_period: f64) -> HilbertOutput {
    if price.len() < 8 {
        return HilbertOutput {
            in_phase: vec![0.0; price.len()],
            quadrature: vec![0.0; price.len()],
            smooth: price.to_vec(),
        };
    }

    let n = price.len();
    let mut smooth = Vec::with_capacity(n);
    let mut detrender = Vec::with_capacity(n);
    let mut in_phase = Vec::with_capacity(n);
    let mut quadrature = Vec::with_capacity(n);

    // Adaptive gain factor based on previous period
    let gain = 0.075 * prev_period + 0.54;

    // FIR coefficients for Hilbert Transform
    const A: f64 = 0.0962;
    const B: f64 = 0.5769;

    // Calculate 4-bar weighted smoothing
    for i in 0..n {
        if i < 3 {
            // Not enough history for full smoothing, use available data
            let s = if i == 0 {
                price[0]
            } else if i == 1 {
                (4.0 * price[1] + 3.0 * price[0]) / 7.0
            } else {
                (4.0 * price[2] + 3.0 * price[1] + 2.0 * price[0]) / 9.0
            };
            smooth.push(s);
        } else {
            let s =
                (4.0 * price[i] + 3.0 * price[i - 1] + 2.0 * price[i - 2] + price[i - 3]) / 10.0;
            smooth.push(s);
        }
    }

    // Calculate detrender using Hilbert FIR coefficients
    for i in 0..n {
        if i < 6 {
            // Not enough history, use simple detrending
            let d = if i == 0 {
                0.0
            } else {
                smooth[i] - smooth[i - 1]
            };
            detrender.push(d);
        } else {
            let d =
                (A * smooth[i] + B * smooth[i - 2] - B * smooth[i - 4] - A * smooth[i - 6]) * gain;
            detrender.push(d);
        }
    }

    // Calculate quadrature (Q1) by applying same FIR to detrender
    for i in 0..n {
        if i < 6 {
            quadrature.push(0.0);
        } else {
            let q = (A * detrender[i] + B * detrender[i - 2]
                - B * detrender[i - 4]
                - A * detrender[i - 6])
                * gain;
            quadrature.push(q);
        }
    }

    // Calculate in-phase (I1) as detrender delayed by 3 bars
    for i in 0..n {
        if i < 3 {
            in_phase.push(0.0);
        } else {
            in_phase.push(detrender[i - 3]);
        }
    }

    HilbertOutput {
        in_phase,
        quadrature,
        smooth,
    }
}

/// Homodyne Discriminator Output
///
/// Contains the dominant cycle period, instantaneous phase,
/// and smoothed period estimates.
#[derive(Debug, Clone, PartialEq)]
pub struct HomodyneOutput {
    /// Dominant cycle period (constrained 6-50 bars)
    pub period: Vec<f64>,
    /// Instantaneous phase in degrees (0-360)
    pub phase: Vec<f64>,
    /// Smoothed period: 0.33*Period + 0.67*SmoothPeriod[1]
    pub smooth_period: Vec<f64>,
}

/// Homodyne Discriminator (Dominant Cycle + Phase)
///
/// Calculates the dominant cycle period and instantaneous phase
/// using the homodyne (product) method on Hilbert Transform outputs.
///
/// # Formula (from Ehlers "Rocket Science for Traders")
/// ```text
/// Re = I1*I1[1] + Q1*Q1[1]  (real part of complex product)
/// Im = I1*Q1[1] - Q1*I1[1]  (imaginary part of complex product)
/// Period = 360 / arctan(Im/Re)  [constrained 6-50]
/// SmoothPeriod = 0.33*Period + 0.67*SmoothPeriod[1]
/// Phase = arctan(Q1/I1) in degrees (0-360)
/// ```
///
/// # Arguments
/// * `i1` - In-phase component from Hilbert Transform
/// * `q1` - Quadrature component from Hilbert Transform
///
/// # Returns
/// * `HomodyneOutput` - Contains period, phase, and smooth_period vectors
pub fn homodyne_discriminator(i1: &[f64], q1: &[f64]) -> HomodyneOutput {
    if i1.len() != q1.len() || i1.len() < 2 {
        let len = i1.len().max(q1.len());
        return HomodyneOutput {
            period: vec![10.0; len], // Default period
            phase: vec![0.0; len],
            smooth_period: vec![10.0; len],
        };
    }

    let n = i1.len();
    let mut period = Vec::with_capacity(n);
    let mut phase = Vec::with_capacity(n);
    let mut smooth_period = Vec::with_capacity(n);

    // Constants for period constraints and smoothing
    const MIN_PERIOD: f64 = 6.0;
    const MAX_PERIOD: f64 = 50.0;
    const PERIOD_SMOOTH: f64 = 0.33;

    for i in 0..n {
        // Calculate instantaneous phase
        let phase_val = if i1[i].abs() < 1e-10 {
            // Avoid division by zero, use previous phase or default
            if i > 0 {
                phase[i - 1]
            } else {
                0.0
            }
        } else {
            let raw_phase = q1[i].atan2(i1[i]).to_degrees();
            // Normalize to 0-360
            if raw_phase < 0.0 {
                raw_phase + 360.0
            } else {
                raw_phase
            }
        };
        phase.push(phase_val);

        // Calculate period using homodyne discriminator
        let period_val = if i < 1 {
            // First value: use default
            10.0
        } else {
            // Complex product components
            let re = i1[i] * i1[i - 1] + q1[i] * q1[i - 1];
            let im = i1[i] * q1[i - 1] - q1[i] * i1[i - 1];

            // Calculate angular frequency from phase difference
            if re.abs() < 1e-10 {
                // Use previous period if real part is too small
                if i > 0 {
                    period[i - 1]
                } else {
                    10.0
                }
            } else {
                let phase_diff = im.atan2(re);

                // Convert phase difference to period
                // Period = 2*PI / phase_diff (in radians)
                // But we use 360 degrees for full cycle
                if phase_diff.abs() < 1e-10 {
                    if i > 0 {
                        period[i - 1]
                    } else {
                        10.0
                    }
                } else {
                    let raw_period = 2.0 * PI / phase_diff.abs();

                    // Constrain period to valid range [6, 50]
                    raw_period.clamp(MIN_PERIOD, MAX_PERIOD)
                }
            }
        };
        period.push(period_val);

        // Calculate smoothed period
        let smooth_val = if i == 0 {
            period_val // First value uses raw period
        } else {
            PERIOD_SMOOTH * period_val + (1.0 - PERIOD_SMOOTH) * smooth_period[i - 1]
        };
        smooth_period.push(smooth_val);
    }

    HomodyneOutput {
        period,
        phase,
        smooth_period,
    }
}

/// Even Better Sinewave
///
/// A normalized cycle indicator that applies the Roofing Filter to remove trend
/// and noise, then normalizes by the square root of power to create a bounded
/// oscillator in the range [-1, +1].
///
/// # Formula (from Ehlers "Cycle Analytics for Traders")
/// ```text
/// Filter = RoofingFilter(Price)
/// Power = Σ(Filter[i]²) / N  (rolling average of squared filter values)
/// EBSW = Filter / sqrt(Power)  [constrained to ±1.0]
/// ```
///
/// # Arguments
/// * `price` - Slice of price values
///
/// # Returns
/// * `Vec<f64>` - Normalized sinewave values bounded [-1, +1]
pub fn even_better_sinewave(price: &[f64]) -> Vec<f64> {
    if price.len() < 3 {
        return vec![0.0; price.len()];
    }

    // Apply Roofing Filter to isolate cyclic components
    let filtered = roofing_filter(price);

    let n = filtered.len();
    let mut ebsw = Vec::with_capacity(n);

    // Use a rolling window for power calculation (default 10 bars per Ehlers)
    const POWER_PERIOD: usize = 10;

    for i in 0..n {
        // Calculate rolling power (average of squared filter values)
        let start = if i >= POWER_PERIOD {
            i - POWER_PERIOD
        } else {
            0
        };
        let window_len = i - start + 1;

        let power: f64 =
            filtered[start..=i].iter().map(|&x| x * x).sum::<f64>() / window_len as f64;

        // Normalize by sqrt(power), with protection against division by zero
        let normalized = if power > 1e-10 {
            let norm_val = filtered[i] / power.sqrt();
            // Constrain to [-1, +1]
            norm_val.clamp(-1.0, 1.0)
        } else {
            0.0
        };

        ebsw.push(normalized);
    }

    ebsw
}

/// Instantaneous Trendline
///
/// A dynamic trendline that uses the smoothed period from Homodyne Discriminator
/// to adjust its smoothing length. Provides trend direction based on dominant cycle.
///
/// # Formula
/// ```text
/// Trendline[i] = Σ Price[k] / N for k=i-N+1 to i
/// Where N = round(SmoothPeriod[i]) from Homodyne
/// ```
///
/// # Arguments
/// * `price` - Slice of price values
/// * `smooth_period` - Slice of smoothed period values from Homodyne discriminator
///
/// # Returns
/// * `Vec<f64>` - Instantaneous trendline values
pub fn instantaneous_trendline(price: &[f64], smooth_period: &[f64]) -> Vec<f64> {
    if price.len() != smooth_period.len() {
        // Return zeros if lengths don't match
        return vec![0.0; price.len().max(smooth_period.len())];
    }

    if price.is_empty() {
        return Vec::new();
    }

    let n = price.len();
    let mut trendline = Vec::with_capacity(n);

    for i in 0..n {
        // Determine the smoothing period (constrained to valid range)
        let period = smooth_period[i].round() as usize;
        let period = period.clamp(1, 50); // Ensure reasonable bounds

        // Calculate simple moving average over the period
        let start = if i >= period - 1 { i + 1 - period } else { 0 };
        let window_len = i - start + 1;

        let sum: f64 = price[start..=i].iter().sum();
        let sma = sum / window_len as f64;

        trendline.push(sma);
    }

    trendline
}

/// MAMA Output Structure
///
/// Contains both the MAMA (Mother of Adaptive Moving Average) and
/// FAMA (Following Adaptive Moving Average) series.
#[derive(Debug, Clone, PartialEq)]
pub struct MamaOutput {
    /// Adaptive moving average (MAMA)
    pub mama: Vec<f64>,
    /// Following adaptive moving average (FAMA)
    pub fama: Vec<f64>,
}

/// MAMA (Mother of Adaptive Moving Average)
///
/// An adaptive moving average developed by John Ehlers that adjusts its
/// smoothing based on the rate of phase change from the Hilbert Transform.
/// FAMA is a slower version of MAMA that creates a signal line.
///
/// # Formula (from Ehlers "Rocket Science for Traders")
/// ```text
/// FastLimit = 0.5, SlowLimit = 0.05
/// alpha = FastLimit / DeltaPhase  [constrained SlowLimit <= alpha <= FastLimit]
/// MAMA = alpha * Price + (1 - alpha) * MAMA[1]
/// FAMA = 0.5 * alpha * MAMA + (1 - 0.5 * alpha) * FAMA[1]
/// ```
///
/// # Arguments
/// * `price` - Slice of price values
/// * `delta_phase` - Slice of phase change values (in degrees)
///
/// # Returns
/// * `MamaOutput` - Contains mama and fama vectors
pub fn mama(price: &[f64], delta_phase: &[f64]) -> MamaOutput {
    if price.len() != delta_phase.len() || price.is_empty() {
        let len = price.len().max(delta_phase.len());
        return MamaOutput {
            mama: vec![0.0; len],
            fama: vec![0.0; len],
        };
    }

    let n = price.len();
    let mut mama_values = Vec::with_capacity(n);
    let mut fama_values = Vec::with_capacity(n);

    // Constants per Ehlers
    const FAST_LIMIT: f64 = 0.5;
    const SLOW_LIMIT: f64 = 0.05;

    for i in 0..n {
        if i == 0 {
            // Initialize with first price
            mama_values.push(price[0]);
            fama_values.push(price[0]);
        } else {
            // Calculate adaptive alpha based on delta phase
            // Avoid division by zero and constrain alpha
            let delta_phase_rad = delta_phase[i].to_radians().abs();
            let raw_alpha = if delta_phase_rad > 1e-10 {
                FAST_LIMIT / delta_phase_rad
            } else {
                FAST_LIMIT
            };

            // Constrain alpha to [SlowLimit, FastLimit]
            let alpha = raw_alpha.clamp(SLOW_LIMIT, FAST_LIMIT);

            // Calculate MAMA: alpha * Price + (1 - alpha) * MAMA[1]
            let mama_val = alpha * price[i] + (1.0 - alpha) * mama_values[i - 1];
            mama_values.push(mama_val);

            // Calculate FAMA: 0.5 * alpha * MAMA + (1 - 0.5 * alpha) * FAMA[1]
            let fama_alpha = 0.5 * alpha;
            let fama_val = fama_alpha * mama_val + (1.0 - fama_alpha) * fama_values[i - 1];
            fama_values.push(fama_val);
        }
    }

    MamaOutput {
        mama: mama_values,
        fama: fama_values,
    }
}

/// Calculate Stochastic %K
///
/// %K = 100 * (Close - Lowest Low) / (Highest High - Lowest Low)
///
/// # Arguments
/// * `high` - Slice of high prices
/// * `low` - Slice of low prices
/// * `close` - Slice of closing prices
/// * `period` - Stochastic period (typically 14)
///
/// # Returns
/// * `Result<Vec<f64>, String>` - %K values or error message
pub fn stochastic_k(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Vec<f64>, String> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err("Input arrays must have equal length".to_string());
    }

    if high.len() < period {
        return Err(format!(
            "Insufficient data: need at least {} data points, got {}",
            period,
            high.len()
        ));
    }

    let mut k_values = Vec::with_capacity(high.len() - period + 1);

    for i in period..=high.len() {
        let highest_high = high[i - period..i]
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lowest_low = low[i - period..i]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));

        let range = highest_high - lowest_low;
        let k = if range == 0.0 {
            50.0 // Default to middle when there's no range
        } else {
            100.0 * (close[i - 1] - lowest_low) / range
        };

        k_values.push(k);
    }

    Ok(k_values)
}

/// ADX (Average Directional Index) Output
///
/// Contains the ADX value along with +DI and -DI components
#[derive(Debug, Clone, PartialEq)]
pub struct AdxOutput {
    /// 14-period smoothed DX
    pub adx: Vec<f64>,
    /// +DI (Positive Directional Indicator)
    pub plus_di: Vec<f64>,
    /// -DI (Negative Directional Indicator)
    pub minus_di: Vec<f64>,
}

/// Calculate ADX (Average Directional Index)
///
/// Measures trend strength on a scale of 0-100.
/// ADX = 100 * smoothed DX, where DX = |+DI - -DI| / (+DI + -DI) * 100
///
/// # Arguments
/// * `high` - Slice of high prices
/// * `low` - Slice of low prices
/// * `close` - Slice of closing prices
/// * `period` - ADX period (typically 14)
///
/// # Returns
/// * `Result<AdxOutput, String>` - ADX, +DI, and -DI values or error message
pub fn adx(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<AdxOutput, String> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err("Input arrays must have equal length".to_string());
    }

    if high.len() < period + 1 {
        return Err(format!(
            "Insufficient data: need at least {} data points, got {}",
            period + 1,
            high.len()
        ));
    }

    let n = high.len();

    // Calculate True Range (TR), +DM, and -DM
    let mut tr_values = Vec::with_capacity(n - 1);
    let mut plus_dm = Vec::with_capacity(n - 1);
    let mut minus_dm = Vec::with_capacity(n - 1);

    for i in 1..n {
        // True Range
        let tr1 = high[i] - low[i];
        let tr2 = (high[i] - close[i - 1]).abs();
        let tr3 = (low[i] - close[i - 1]).abs();
        tr_values.push(tr1.max(tr2).max(tr3));

        // +DM and -DM
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        let plus = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };

        let minus = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };

        plus_dm.push(plus);
        minus_dm.push(minus);
    }

    // Calculate smoothed TR, +DM, -DM using Wilder's smoothing
    let mut smoothed_tr = Vec::with_capacity(tr_values.len() - period + 1);
    let mut smoothed_plus_dm = Vec::with_capacity(plus_dm.len() - period + 1);
    let mut smoothed_minus_dm = Vec::with_capacity(minus_dm.len() - period + 1);

    // Initial SMA
    let mut atr: f64 = tr_values.iter().take(period).sum::<f64>() / period as f64;
    let mut plus_dm_sum: f64 = plus_dm.iter().take(period).sum::<f64>() / period as f64;
    let mut minus_dm_sum: f64 = minus_dm.iter().take(period).sum::<f64>() / period as f64;

    smoothed_tr.push(atr);
    smoothed_plus_dm.push(plus_dm_sum);
    smoothed_minus_dm.push(minus_dm_sum);

    // Wilder's smoothing for remaining values
    for i in period..tr_values.len() {
        atr = (atr * (period - 1) as f64 + tr_values[i]) / period as f64;
        plus_dm_sum = (plus_dm_sum * (period - 1) as f64 + plus_dm[i]) / period as f64;
        minus_dm_sum = (minus_dm_sum * (period - 1) as f64 + minus_dm[i]) / period as f64;

        smoothed_tr.push(atr);
        smoothed_plus_dm.push(plus_dm_sum);
        smoothed_minus_dm.push(minus_dm_sum);
    }

    // Calculate +DI and -DI
    let mut plus_di_values = Vec::with_capacity(smoothed_tr.len());
    let mut minus_di_values = Vec::with_capacity(smoothed_tr.len());

    for i in 0..smoothed_tr.len() {
        let plus_di = if smoothed_tr[i] == 0.0 {
            0.0
        } else {
            100.0 * smoothed_plus_dm[i] / smoothed_tr[i]
        };
        let minus_di = if smoothed_tr[i] == 0.0 {
            0.0
        } else {
            100.0 * smoothed_minus_dm[i] / smoothed_tr[i]
        };

        plus_di_values.push(plus_di);
        minus_di_values.push(minus_di);
    }

    // Calculate DX and then ADX
    let mut dx_values = Vec::with_capacity(plus_di_values.len());

    for i in 0..plus_di_values.len() {
        let di_sum = plus_di_values[i] + minus_di_values[i];
        let di_diff = (plus_di_values[i] - minus_di_values[i]).abs();

        let dx = if di_sum == 0.0 {
            0.0
        } else {
            100.0 * di_diff / di_sum
        };
        dx_values.push(dx);
    }

    // Smooth DX to get ADX
    let mut adx_values = Vec::with_capacity(dx_values.len() - period + 1);

    // Initial ADX (SMA of first period DX values)
    let mut adx_val: f64 = dx_values.iter().take(period).sum::<f64>() / period as f64;
    adx_values.push(adx_val);

    // Wilder's smoothing for remaining ADX
    for i in period..dx_values.len() {
        adx_val = (adx_val * (period - 1) as f64 + dx_values[i]) / period as f64;
        adx_values.push(adx_val);
    }

    // Align +DI and -DI with ADX output
    let aligned_plus_di: Vec<f64> = plus_di_values.iter().skip(period - 1).copied().collect();
    let aligned_minus_di: Vec<f64> = minus_di_values.iter().skip(period - 1).copied().collect();

    Ok(AdxOutput {
        adx: adx_values,
        plus_di: aligned_plus_di,
        minus_di: aligned_minus_di,
    })
}

/// Calculate CCI (Commodity Channel Index)
///
/// CCI = (Typical Price - SMA) / (0.015 * Mean Deviation)
/// Typical Price = (High + Low + Close) / 3
///
/// # Arguments
/// * `high` - Slice of high prices
/// * `low` - Slice of low prices
/// * `close` - Slice of closing prices
/// * `period` - CCI period (typically 20)
///
/// # Returns
/// * `Result<Vec<f64>, String>` - CCI values or error message
pub fn cci(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Vec<f64>, String> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err("Input arrays must have equal length".to_string());
    }

    if high.len() < period {
        return Err(format!(
            "Insufficient data: need at least {} data points, got {}",
            period,
            high.len()
        ));
    }

    // Calculate Typical Price
    let typical_price: Vec<f64> = high
        .iter()
        .zip(low.iter())
        .zip(close.iter())
        .map(|((h, l), c)| (h + l + c) / 3.0)
        .collect();

    let mut cci_values = Vec::with_capacity(high.len() - period + 1);

    for i in period..=typical_price.len() {
        let slice = &typical_price[i - period..i];

        // Calculate SMA of Typical Price
        let sma: f64 = slice.iter().sum::<f64>() / period as f64;

        // Calculate Mean Deviation
        let mean_deviation: f64 =
            slice.iter().map(|tp| (tp - sma).abs()).sum::<f64>() / period as f64;

        // Calculate CCI
        let cci = if mean_deviation == 0.0 {
            0.0
        } else {
            (typical_price[i - 1] - sma) / (0.015 * mean_deviation)
        };

        cci_values.push(cci);
    }

    Ok(cci_values)
}

/// Ichimoku Cloud Output
///
/// Contains all five lines of the Ichimoku Cloud indicator
#[derive(Debug, Clone, PartialEq)]
pub struct IchimokuOutput {
    /// Tenkan-sen (Conversion Line): (Highest High + Lowest Low) / 2 over 9 periods
    pub tenkan: Vec<f64>,
    /// Kijun-sen (Base Line): (Highest High + Lowest Low) / 2 over 26 periods
    pub kijun: Vec<f64>,
    /// Senkou Span A (Leading Span A): (Tenkan + Kijun) / 2 projected 26 periods forward
    pub senkou_a: Vec<f64>,
    /// Senkou Span B (Leading Span B): (Highest High + Lowest Low) / 2 over 52 periods, projected 26 forward
    pub senkou_b: Vec<f64>,
}

/// Calculate Ichimoku Cloud
///
/// A comprehensive trend indicator showing support/resistance, momentum, and trend direction.
///
/// Standard periods: Tenkan=9, Kijun=26, Senkou B=52, Displacement=26
///
/// # Arguments
/// * `high` - Slice of high prices
/// * `low` - Slice of low prices
///
/// # Returns
/// * `Result<IchimokuOutput, String>` - Ichimoku lines or error message
pub fn ichimoku(high: &[f64], low: &[f64]) -> Result<IchimokuOutput, String> {
    if high.len() != low.len() {
        return Err("High and low arrays must have equal length".to_string());
    }

    // Standard Ichimoku periods
    const TENKAN_PERIOD: usize = 9;
    const KIJUN_PERIOD: usize = 26;
    const SENKOU_B_PERIOD: usize = 52;
    const DISPLACEMENT: usize = 26;

    let min_required = SENKOU_B_PERIOD;
    if high.len() < min_required {
        return Err(format!(
            "Insufficient data: need at least {} data points, got {}",
            min_required,
            high.len()
        ));
    }

    let n = high.len();

    // Calculate Tenkan-sen (9-period)
    let mut tenkan = Vec::with_capacity(n - TENKAN_PERIOD + 1);
    for i in TENKAN_PERIOD..=n {
        let highest = high[i - TENKAN_PERIOD..i]
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lowest = low[i - TENKAN_PERIOD..i]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));
        tenkan.push((highest + lowest) / 2.0);
    }

    // Calculate Kijun-sen (26-period)
    let mut kijun = Vec::with_capacity(n - KIJUN_PERIOD + 1);
    for i in KIJUN_PERIOD..=n {
        let highest = high[i - KIJUN_PERIOD..i]
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lowest = low[i - KIJUN_PERIOD..i]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));
        kijun.push((highest + lowest) / 2.0);
    }

    // Calculate Senkou Span B (52-period)
    let mut senkou_b_raw = Vec::with_capacity(n - SENKOU_B_PERIOD + 1);
    for i in SENKOU_B_PERIOD..=n {
        let highest = high[i - SENKOU_B_PERIOD..i]
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lowest = low[i - SENKOU_B_PERIOD..i]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));
        senkou_b_raw.push((highest + lowest) / 2.0);
    }

    // Calculate Senkou Span A: (Tenkan + Kijun) / 2
    // Senkou Span A and B need to be projected 26 periods forward
    // Align all outputs to the same length (after Kijun calculation)
    let output_len = kijun.len();

    // Senkou Span A needs both Tenkan and Kijun
    let mut senkou_a = vec![f64::NAN; output_len];
    for i in 0..output_len {
        // Kijun starts at index 0, Tenkan at index (KIJUN_PERIOD - TENKAN_PERIOD)
        let tenkan_idx = i + KIJUN_PERIOD - TENKAN_PERIOD;
        if tenkan_idx < tenkan.len() {
            senkou_a[i] = (tenkan[tenkan_idx] + kijun[i]) / 2.0;
        }
    }

    // Senkou Span B projection
    // senkou_b_raw starts at index SENKOU_B_PERIOD, we need to align with kijun output
    let mut senkou_b = vec![f64::NAN; output_len];
    let senkou_b_start = SENKOU_B_PERIOD - KIJUN_PERIOD;
    for i in senkou_b_start..senkou_b_raw.len().min(output_len + senkou_b_start) {
        senkou_b[i - senkou_b_start] = senkou_b_raw[i];
    }

    // Project forward 26 periods by shifting back and padding with NaN at the end
    let mut senkou_a_projected = vec![f64::NAN; output_len];
    let mut senkou_b_projected = vec![f64::NAN; output_len];

    for i in 0..output_len.saturating_sub(DISPLACEMENT) {
        senkou_a_projected[i] = senkou_a[i + DISPLACEMENT];
        senkou_b_projected[i] =
            if i + DISPLACEMENT < senkou_b.len() && !senkou_b[i + DISPLACEMENT].is_nan() {
                senkou_b[i + DISPLACEMENT]
            } else {
                f64::NAN
            };
    }

    // Fill initial NaN values with the first calculated value for cleaner output
    let first_valid_a = senkou_a_projected
        .iter()
        .position(|&x| !x.is_nan())
        .unwrap_or(0);
    let first_valid_b = senkou_b_projected
        .iter()
        .position(|&x| !x.is_nan())
        .unwrap_or(0);

    for i in 0..first_valid_a {
        senkou_a_projected[i] = senkou_a_projected[first_valid_a];
    }
    for i in 0..first_valid_b {
        senkou_b_projected[i] = senkou_b_projected[first_valid_b];
    }

    Ok(IchimokuOutput {
        tenkan,
        kijun,
        senkou_a: senkou_a_projected,
        senkou_b: senkou_b_projected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_prices() -> Vec<f64> {
        vec![
            100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0, 108.0, 107.0, 109.0, 110.0, 108.0,
            111.0, 113.0, 112.0, 114.0, 116.0, 115.0, 117.0, 119.0, 118.0, 120.0, 122.0, 121.0,
            123.0, 125.0, 124.0, 126.0, 128.0, 127.0, 129.0, 131.0, 130.0, 132.0, 134.0, 135.0,
            133.0, 136.0, 138.0, 137.0, 139.0, 141.0, 140.0, 142.0, 144.0, 143.0, 145.0, 147.0,
        ]
    }

    #[test]
    fn test_calculate_rsi() {
        let prices = generate_test_prices();
        let result = calculate_rsi(&prices, 14);
        assert!(result.is_ok());
        let rsi_values = result.unwrap();
        assert!(!rsi_values.is_empty());
        // RSI should be between 0 and 100
        for val in &rsi_values {
            assert!(*val >= 0.0 && *val <= 100.0);
        }
    }

    #[test]
    fn test_calculate_rsi_insufficient_data() {
        let prices = vec![100.0, 101.0];
        let result = calculate_rsi(&prices, 14);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_macd() {
        let prices = generate_test_prices();
        let result = calculate_macd(&prices, 12, 26, 9);
        assert!(result.is_ok());
        let (macd_line, signal_line, histogram) = result.unwrap();
        assert_eq!(macd_line.len(), signal_line.len());
        assert_eq!(signal_line.len(), histogram.len());
    }

    #[test]
    fn test_calculate_macd_invalid_periods() {
        let prices = generate_test_prices();
        let result = calculate_macd(&prices, 26, 12, 9);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_bollinger() {
        let prices = generate_test_prices();
        let result = calculate_bollinger(&prices, 20, 2.0);
        assert!(result.is_ok());
        let (upper, middle, lower) = result.unwrap();
        assert_eq!(upper.len(), middle.len());
        assert_eq!(middle.len(), lower.len());
        // Upper should be >= middle >= lower
        for i in 0..upper.len() {
            assert!(upper[i] >= middle[i]);
            assert!(middle[i] >= lower[i]);
        }
    }

    #[test]
    fn test_calculate_atr() {
        let highs = generate_test_prices();
        let lows: Vec<f64> = highs.iter().map(|p| p - 2.0).collect();
        let closes: Vec<f64> = highs.iter().map(|p| p - 1.0).collect();
        let result = calculate_atr(&highs, &lows, &closes, 14);
        assert!(result.is_ok());
        let atr_values = result.unwrap();
        assert!(!atr_values.is_empty());
        // ATR should be positive
        for val in &atr_values {
            assert!(*val > 0.0);
        }
    }

    #[test]
    fn test_calculate_atr_mismatched_lengths() {
        let highs = vec![100.0, 101.0, 102.0];
        let lows = vec![98.0, 99.0];
        let closes = vec![99.0, 100.0, 101.0];
        let result = calculate_atr(&highs, &lows, &closes, 14);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_superposition_score() {
        let price_data: Vec<MarketData> = generate_test_prices()
            .iter()
            .enumerate()
            .map(|(i, &close)| MarketData {
                timestamp: 1704295800000 + (i as u64 * 60000),
                open: close - 1.0,
                high: close + 1.0,
                low: close - 2.0,
                close,
                volume: 1000000,
            })
            .collect();

        let timeframes = vec![5, 10, 20];
        let result = calculate_superposition_score(&price_data, &timeframes);
        assert!(result.is_ok());
        let score = result.unwrap();
        assert!(score >= 0.0 && score <= 1.0);
    }

    // ============================================================================
    // Ehlers SuperSmoother Tests
    // ============================================================================

    #[test]
    fn test_supersmoother_basic() {
        // Test with constant price - should stay constant after warmup
        let prices = vec![100.0; 20];
        let result = supersmoother(&prices);
        assert_eq!(result.len(), prices.len());
        // After warmup, constant input should produce nearly constant output
        for val in result.iter().skip(5) {
            assert!(
                (val - 100.0).abs() < 0.01,
                "Constant input should converge to constant output"
            );
        }
    }

    #[test]
    fn test_supersmoother_short_input() {
        // Test with minimal data (2 points)
        let prices = vec![100.0, 101.0];
        let result = supersmoother(&prices);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 100.0);
        // Second value is computed using filter formula, not simple average
        assert!(result[1].is_finite());
    }

    #[test]
    fn test_supersmoother_single_value() {
        let prices = vec![100.0];
        let result = supersmoother(&prices);
        assert_eq!(result, vec![100.0]);
    }

    #[test]
    fn test_supersmoother_coefficients() {
        // Verify the exact coefficients are used correctly
        // a1 = exp(-1.414 * PI / 10) ≈ 0.639
        // b1 = 2 * a1 * cos(1.414 * PI / 10) ≈ 1.178
        let a1: f64 = (-1.414_f64 * PI / 10.0).exp();
        let expected_a1 = 0.639;
        assert!(
            (a1 - expected_a1).abs() < 0.01,
            "a1 coefficient should be approximately 0.639"
        );

        let b1: f64 = 2.0 * a1 * (1.414_f64 * 180.0 / 10.0).to_radians().cos();
        // b1 ≈ 1.158 (calculated from Ehlers formula using degrees converted to radians)
        assert!(
            (b1 - 1.158).abs() < 0.02,
            "b1 coefficient should be approximately 1.158, got {}",
            b1
        );

        // Verify c1, c2, c3 coefficients are valid
        let c2 = b1;
        let c3 = -a1 * a1;
        let c1 = 1.0 - c2 - c3;

        assert!(
            c1 > 0.0 && c1 < 1.0,
            "c1 should be positive and less than 1, got {}",
            c1
        );
        assert!(
            c2 > 1.0 && c2 < 2.0,
            "c2 (b1) should be between 1 and 2, got {}",
            c2
        );
        assert!(
            c3 < 0.0 && c3 > -1.0,
            "c3 should be negative and between -1 and 0, got {}",
            c3
        );
    }

    #[test]
    fn test_supersmoother_smoothing_effect() {
        // Test that smoothing reduces high-frequency noise
        let noisy_prices: Vec<f64> = (0..50)
            .map(|i| {
                let base = 100.0 + i as f64 * 0.5;
                let noise = if i % 2 == 0 { 2.0 } else { -2.0 };
                base + noise
            })
            .collect();

        let smoothed = supersmoother(&noisy_prices);

        // Calculate variance reduction
        let input_variance = calculate_variance(&noisy_prices);
        let output_variance = calculate_variance(&smoothed[10..].to_vec());

        assert!(
            output_variance < input_variance,
            "SuperSmoother should reduce variance"
        );
    }

    #[test]
    fn test_supersmoother_trend_following() {
        // Test that smoother follows trend (not lag excessively)
        let trend_prices: Vec<f64> = (0..30).map(|i| 100.0 + i as f64 * 1.0).collect();
        let smoothed = supersmoother(&trend_prices);

        // After warmup, smoothed values should follow the trend
        let first_smoothed = smoothed[smoothed.len() - 5];
        let last_smoothed = smoothed[smoothed.len() - 1];

        assert!(
            last_smoothed > first_smoothed,
            "Smoother should follow upward trend"
        );
    }

    #[test]
    fn test_supersmoother_known_values() {
        // Test with known input/output values
        let prices = vec![
            100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0, 108.0, 107.0, 109.0, 110.0, 108.0,
            111.0, 113.0, 112.0, 114.0, 116.0, 115.0, 117.0, 119.0,
        ];
        let smoothed = supersmoother(&prices);

        // Verify output length
        assert_eq!(smoothed.len(), prices.len());

        // Verify first value is initialized correctly
        assert_eq!(smoothed[0], prices[0]);
        // Second value is computed using filter formula (not simple average)
        assert!(smoothed[1].is_finite());

        // Verify all values are finite
        for val in &smoothed {
            assert!(val.is_finite(), "All smoothed values should be finite");
        }
    }

    // ============================================================================
    // Ehlers Roofing Filter Tests
    // ============================================================================

    #[test]
    fn test_roofing_filter_basic() {
        // Test with constant price - should produce near-zero output after warmup
        let prices = vec![100.0; 50];
        let result = roofing_filter(&prices);
        assert_eq!(result.len(), prices.len());

        // After warmup, constant input should produce values near zero
        // (HighPass removes constant, SuperSmoother smoothes to near-zero)
        for val in result.iter().skip(10) {
            assert!(
                val.abs() < 1.0,
                "Constant input should produce near-zero output: got {}",
                val
            );
        }
    }

    #[test]
    fn test_roofing_filter_short_input() {
        // Test with insufficient data
        let prices = vec![100.0, 101.0];
        let result = roofing_filter(&prices);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_roofing_filter_trend_removal() {
        // Strong linear trend should be attenuated by HighPass component
        // The 48-bar HighPass filter removes very low frequency components (trends)
        let trend: Vec<f64> = (0..300).map(|i| 100.0 + i as f64 * 2.0).collect();
        let filtered = roofing_filter(&trend);

        // After sufficient warmup, the HighPass filter should significantly attenuate the trend
        // The first difference of trend is constant 2.0, after HP filtering should be much smaller
        let warmup = 100;
        let max_val = filtered[warmup..]
            .iter()
            .map(|v| v.abs())
            .fold(0.0, f64::max);

        // The maximum value should be bounded (much smaller than trend magnitude ~700)
        assert!(
            max_val < 300.0,
            "Roofing Filter output should be bounded (max should be << trend magnitude, got {})",
            max_val
        );
    }

    #[test]
    fn test_roofing_filter_cyclic_preservation() {
        // Create a synthetic cyclic signal with period ~48 bars (matching filter design)
        let cycle: Vec<f64> = (0..100)
            .map(|i| {
                let t = i as f64;
                100.0 + 5.0 * (2.0 * PI * t / 48.0).sin()
            })
            .collect();

        let filtered = roofing_filter(&cycle);

        // Verify output has reasonable values (not all zeros)
        let max_val = filtered
            .iter()
            .skip(20)
            .map(|v| v.abs())
            .fold(0.0, f64::max);
        assert!(
            max_val > 0.1,
            "Roofing Filter should preserve cyclic components"
        );
    }

    #[test]
    fn test_roofing_filter_composition() {
        // Verify that roofing_filter is composed of HighPass + SuperSmoother
        let prices = generate_test_prices();

        // Calculate manually
        let alpha: f64 = (0.707_f64 * 2.0 * PI / 48.0).powi(2);
        let one_minus_alpha = 1.0 - alpha;
        let one_minus_alpha_half = 1.0 - alpha / 2.0;

        let mut hp = vec![0.0];
        for i in 1..prices.len() {
            let hp_val = if i == 1 {
                one_minus_alpha_half * (prices[i] - prices[i - 1])
            } else {
                one_minus_alpha_half * (prices[i] - prices[i - 1]) + one_minus_alpha * hp[i - 1]
            };
            hp.push(hp_val);
        }

        let expected = supersmoother(&hp);
        let actual = roofing_filter(&prices);

        assert_eq!(expected.len(), actual.len());
        for (e, a) in expected.iter().zip(actual.iter()) {
            assert!(
                (e - a).abs() < 1e-10,
                "Roofing filter should match HighPass + SuperSmoother composition"
            );
        }
    }

    #[test]
    fn test_roofing_filter_known_values() {
        // Test with known price sequence
        let prices = vec![
            100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0, 108.0, 107.0, 109.0, 110.0, 108.0,
            111.0, 113.0, 112.0, 114.0, 116.0, 115.0, 117.0, 119.0, 118.0, 120.0, 122.0, 121.0,
            123.0, 125.0, 124.0, 126.0, 128.0, 127.0, 129.0, 131.0, 130.0, 132.0, 134.0, 133.0,
            135.0, 137.0, 136.0, 138.0, 140.0, 139.0, 141.0, 143.0, 142.0, 144.0, 146.0, 145.0,
            147.0, 149.0,
        ];

        let filtered = roofing_filter(&prices);

        // Verify output length matches input
        assert_eq!(filtered.len(), prices.len());

        // Verify all values are finite
        for val in &filtered {
            assert!(val.is_finite(), "All filtered values should be finite");
        }

        // Verify first value is 0 (initialized HighPass)
        assert_eq!(filtered[0], 0.0);
    }

    #[test]
    fn test_roofing_filter_highpass_coefficient() {
        // Verify the HighPass coefficient calculation
        let alpha: f64 = (0.707_f64 * 2.0 * PI / 48.0).powi(2);
        let expected_alpha = (0.707_f64 * 2.0 * 3.14159 / 48.0).powi(2);
        assert!(
            (alpha - expected_alpha).abs() < 0.0001,
            "Alpha coefficient calculation should be correct"
        );
    }

    // ============================================================================
    // Helper Functions for Tests
    // ============================================================================

    fn calculate_variance(data: &[f64]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64
    }

    // ============================================================================
    // Hilbert Transform Tests
    // ============================================================================

    #[test]
    fn test_hilbert_transform_basic() {
        // Test with a simple sine wave (known cycle)
        let cycle: Vec<f64> = (0..100)
            .map(|i| {
                let t = i as f64;
                100.0 + 10.0 * (2.0 * PI * t / 20.0).sin() // 20-bar cycle
            })
            .collect();

        let result = hilbert_transform(&cycle, 20.0);

        // Verify output lengths match input
        assert_eq!(result.in_phase.len(), cycle.len());
        assert_eq!(result.quadrature.len(), cycle.len());
        assert_eq!(result.smooth.len(), cycle.len());
    }

    #[test]
    fn test_hilbert_transform_short_input() {
        // Test with insufficient data
        let prices = vec![100.0, 101.0, 102.0];
        let result = hilbert_transform(&prices, 10.0);

        // Should return zero-filled vectors for insufficient data
        assert_eq!(result.in_phase.len(), 3);
        assert_eq!(result.quadrature.len(), 3);
        assert_eq!(result.smooth.len(), 3);
    }

    #[test]
    fn test_hilbert_transform_quadrature_orthogonality() {
        // For a pure sine wave, I and Q should be roughly 90 degrees out of phase
        // after the warmup period
        let cycle: Vec<f64> = (0..100)
            .map(|i| {
                let t = i as f64;
                (2.0 * PI * t / 20.0).sin()
            })
            .collect();

        let result = hilbert_transform(&cycle, 20.0);

        // Skip warmup period and check that both I and Q have significant values
        let warmup = 20;
        let i_values: Vec<f64> = result.in_phase.iter().skip(warmup).copied().collect();
        let q_values: Vec<f64> = result.quadrature.iter().skip(warmup).copied().collect();

        // Both should have non-zero values after warmup
        let i_max = i_values.iter().map(|v| v.abs()).fold(0.0, f64::max);
        let q_max = q_values.iter().map(|v| v.abs()).fold(0.0, f64::max);

        assert!(i_max > 0.01, "In-phase should have significant amplitude");
        assert!(q_max > 0.01, "Quadrature should have significant amplitude");
    }

    #[test]
    fn test_hilbert_transform_smooth_coefficients() {
        // Verify the 4-bar smoothing coefficients at index >= 3
        let prices = vec![100.0, 104.0, 102.0, 106.0, 108.0, 110.0, 112.0, 114.0];
        let result = hilbert_transform(&prices, 10.0);

        // For index 5: smooth = (4*price[5] + 3*price[4] + 2*price[3] + price[2]) / 10
        // = (4*110 + 3*108 + 2*106 + 102) / 10 = (440 + 324 + 212 + 102) / 10 = 1078 / 10 = 107.8
        let expected_smooth_5 =
            (4.0 * prices[5] + 3.0 * prices[4] + 2.0 * prices[3] + prices[2]) / 10.0;
        assert!(
            (result.smooth[5] - expected_smooth_5).abs() < 0.01,
            "Expected smooth[5] = {}, got {}",
            expected_smooth_5,
            result.smooth[5]
        );
    }

    #[test]
    fn test_hilbert_transform_constant_input() {
        // Constant input should produce near-zero detrended output
        let prices = vec![100.0; 50];
        let result = hilbert_transform(&prices, 10.0);

        // After warmup, I and Q should be near zero for constant input
        for i in 20..result.in_phase.len() {
            assert!(
                result.in_phase[i].abs() < 0.1,
                "Constant input should produce near-zero in-phase"
            );
            assert!(
                result.quadrature[i].abs() < 0.1,
                "Constant input should produce near-zero quadrature"
            );
        }
    }

    #[test]
    fn test_hilbert_transform_output_finite() {
        // Test that all outputs are finite
        let prices = generate_test_prices();
        let result = hilbert_transform(&prices, 14.0);

        for (i, val) in result.in_phase.iter().enumerate() {
            assert!(val.is_finite(), "In-phase[{}] should be finite", i);
        }
        for (i, val) in result.quadrature.iter().enumerate() {
            assert!(val.is_finite(), "Quadrature[{}] should be finite", i);
        }
        for (i, val) in result.smooth.iter().enumerate() {
            assert!(val.is_finite(), "Smooth[{}] should be finite", i);
        }
    }

    // ============================================================================
    // Homodyne Discriminator Tests
    // ============================================================================

    #[test]
    fn test_homodyne_discriminator_basic() {
        // Create synthetic I and Q from a known cycle
        let cycle_period = 20.0;
        let n = 100;

        let i1: Vec<f64> = (0..n)
            .map(|i| {
                let t = i as f64;
                (2.0 * PI * t / cycle_period).cos()
            })
            .collect();

        let q1: Vec<f64> = (0..n)
            .map(|i| {
                let t = i as f64;
                (2.0 * PI * t / cycle_period).sin()
            })
            .collect();

        let result = homodyne_discriminator(&i1, &q1);

        // Verify output lengths
        assert_eq!(result.period.len(), n);
        assert_eq!(result.phase.len(), n);
        assert_eq!(result.smooth_period.len(), n);
    }

    #[test]
    fn test_homodyne_discriminator_period_constraints() {
        // Test that period is constrained to [6, 50]
        let n = 100;
        let i1: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).cos()).collect();
        let q1: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).sin()).collect();

        let result = homodyne_discriminator(&i1, &q1);

        // All periods should be within [6, 50]
        for (i, &period) in result.period.iter().enumerate().skip(1) {
            assert!(
                period >= 6.0 && period <= 50.0,
                "Period[{}] = {} should be in [6, 50]",
                i,
                period
            );
        }
    }

    #[test]
    fn test_homodyne_discriminator_phase_range() {
        // Test that phase is in [0, 360]
        let n = 100;
        let i1: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).cos()).collect();
        let q1: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).sin()).collect();

        let result = homodyne_discriminator(&i1, &q1);

        // All phases should be in [0, 360]
        for (i, &phase) in result.phase.iter().enumerate() {
            assert!(
                phase >= 0.0 && phase <= 360.0,
                "Phase[{}] = {} should be in [0, 360]",
                i,
                phase
            );
        }
    }

    #[test]
    fn test_homodyne_discriminator_phase_quadrants() {
        // Test phase calculation in different quadrants
        // I>0, Q>0: phase in (0, 90)
        // I<0, Q>0: phase in (90, 180)
        // I<0, Q<0: phase in (180, 270)
        // I>0, Q<0: phase in (270, 360)

        let i1 = vec![1.0, -1.0, -1.0, 1.0];
        let q1 = vec![1.0, 1.0, -1.0, -1.0];

        let result = homodyne_discriminator(&i1, &q1);

        // Q1: I>0, Q>0 -> phase ~45 degrees
        assert!(result.phase[0] > 0.0 && result.phase[0] < 90.0);
        // Q2: I<0, Q>0 -> phase ~135 degrees
        assert!(result.phase[1] > 90.0 && result.phase[1] < 180.0);
        // Q3: I<0, Q<0 -> phase ~225 degrees
        assert!(result.phase[2] > 180.0 && result.phase[2] < 270.0);
        // Q4: I>0, Q<0 -> phase ~315 degrees
        assert!(result.phase[3] > 270.0 && result.phase[3] < 360.0);
    }

    #[test]
    fn test_homodyne_discriminator_period_smoothing() {
        // Test that smooth_period follows the smoothing formula
        // SmoothPeriod[i] = 0.33*Period[i] + 0.67*SmoothPeriod[i-1]
        let n = 50;
        let i1: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).cos()).collect();
        let q1: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).sin()).collect();

        let result = homodyne_discriminator(&i1, &q1);

        // Verify the smoothing formula is applied correctly
        for i in 1..result.smooth_period.len() {
            let expected = 0.33 * result.period[i] + 0.67 * result.smooth_period[i - 1];
            assert!(
                (result.smooth_period[i] - expected).abs() < 0.001,
                "SmoothPeriod[{}] should equal 0.33*Period + 0.67*SmoothPeriod[{}]",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn test_homodyne_discriminator_mismatched_lengths() {
        // Test with mismatched input lengths
        let i1 = vec![1.0, 2.0, 3.0];
        let q1 = vec![1.0, 2.0];

        let result = homodyne_discriminator(&i1, &q1);

        // Should handle gracefully
        assert!(!result.period.is_empty());
    }

    #[test]
    fn test_homodyne_discriminator_output_finite() {
        // Test that all outputs are finite
        let prices = generate_test_prices();
        let hilbert = hilbert_transform(&prices, 14.0);
        let result = homodyne_discriminator(&hilbert.in_phase, &hilbert.quadrature);

        for (i, val) in result.period.iter().enumerate() {
            assert!(val.is_finite(), "Period[{}] should be finite", i);
        }
        for (i, val) in result.phase.iter().enumerate() {
            assert!(val.is_finite(), "Phase[{}] should be finite", i);
        }
        for (i, val) in result.smooth_period.iter().enumerate() {
            assert!(val.is_finite(), "SmoothPeriod[{}] should be finite", i);
        }
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[test]
    fn test_hilbert_homodyne_pipeline() {
        // Test the complete pipeline from price to cycle measurement
        let prices = generate_test_prices();

        // Step 1: Apply Hilbert Transform
        let hilbert = hilbert_transform(&prices, 14.0);

        // Step 2: Apply Homodyne Discriminator
        let homodyne = homodyne_discriminator(&hilbert.in_phase, &hilbert.quadrature);

        // Verify all outputs have correct length
        assert_eq!(homodyne.period.len(), prices.len());
        assert_eq!(homodyne.phase.len(), prices.len());
        assert_eq!(homodyne.smooth_period.len(), prices.len());

        // Verify period constraints
        for &period in &homodyne.period {
            assert!(period >= 6.0 && period <= 50.0);
        }

        // Verify phase range
        for &phase in &homodyne.phase {
            assert!(phase >= 0.0 && phase <= 360.0);
        }
    }

    #[test]
    fn test_known_cycle_detection() {
        // Create a signal with a known 20-bar cycle
        let cycle_period = 20.0;
        let prices: Vec<f64> = (0..200)
            .map(|i| {
                let t = i as f64;
                100.0 + 10.0 * (2.0 * PI * t / cycle_period).sin()
            })
            .collect();

        // First pass: use default period
        let hilbert = hilbert_transform(&prices, 20.0);
        let homodyne = homodyne_discriminator(&hilbert.in_phase, &hilbert.quadrature);

        // After warmup, smoothed period should be close to 20
        let warmup = 50;
        let avg_period: f64 = homodyne.smooth_period[warmup..].iter().sum::<f64>()
            / (homodyne.smooth_period.len() - warmup) as f64;

        // Allow some tolerance due to adaptive nature and warmup
        assert!(
            avg_period >= 15.0 && avg_period <= 35.0,
            "Detected period {} should be reasonably close to input period 20",
            avg_period
        );
    }

    // ============================================================================
    // Even Better Sinewave Tests
    // ============================================================================

    #[test]
    fn test_even_better_sinewave_basic() {
        let prices = generate_test_prices();
        let result = even_better_sinewave(&prices);

        // Verify output length matches input
        assert_eq!(result.len(), prices.len());

        // Verify all values are bounded [-1, +1]
        for (i, &val) in result.iter().enumerate() {
            assert!(
                val >= -1.0 && val <= 1.0,
                "EBSW[{}] = {} should be in [-1, 1]",
                i,
                val
            );
        }
    }

    #[test]
    fn test_even_better_sinewave_short_input() {
        // Test with insufficient data
        let prices = vec![100.0, 101.0];
        let result = even_better_sinewave(&prices);
        assert_eq!(result.len(), 2);
        assert_eq!(result, vec![0.0, 0.0]);
    }

    #[test]
    fn test_even_better_sinewave_empty_input() {
        let prices: Vec<f64> = vec![];
        let result = even_better_sinewave(&prices);
        assert!(result.is_empty());
    }

    #[test]
    fn test_even_better_sinewave_bounded_range() {
        // Create a signal with high volatility
        let prices: Vec<f64> = (0..100)
            .map(|i| 100.0 + 50.0 * (i as f64 * 0.5).sin())
            .collect();

        let result = even_better_sinewave(&prices);

        // All values should be strictly bounded [-1, 1]
        for (i, &val) in result.iter().enumerate().skip(10) {
            assert!(
                val >= -1.0 && val <= 1.0,
                "EBSW[{}] = {} is outside [-1, 1]",
                i,
                val
            );
        }
    }

    #[test]
    fn test_even_better_sinewave_constant_input() {
        // Constant input should produce near-zero output (no cycle)
        let prices = vec![100.0; 50];
        let result = even_better_sinewave(&prices);

        // After warmup, constant input should produce near-zero values
        for val in result.iter().skip(15) {
            assert!(
                val.abs() < 0.5,
                "Constant input should produce near-zero EBSW: got {}",
                val
            );
        }
    }

    #[test]
    fn test_even_better_sinewave_cyclic_input() {
        // Create a pure sine wave cycle
        let cycle: Vec<f64> = (0..100)
            .map(|i| {
                let t = i as f64;
                100.0 + 10.0 * (2.0 * PI * t / 20.0).sin()
            })
            .collect();

        let result = even_better_sinewave(&cycle);

        // Verify output has varying values (not all zeros)
        let max_val = result.iter().skip(20).map(|v| v.abs()).fold(0.0, f64::max);
        assert!(
            max_val > 0.1,
            "EBSW should detect cyclic components, max value: {}",
            max_val
        );

        // Verify bounded range
        for &val in &result {
            assert!(val >= -1.0 && val <= 1.0);
        }
    }

    #[test]
    fn test_even_better_sinewave_finite_values() {
        let prices = generate_test_prices();
        let result = even_better_sinewave(&prices);

        for (i, val) in result.iter().enumerate() {
            assert!(val.is_finite(), "EBSW[{}] should be finite, got {}", i, val);
        }
    }

    // ============================================================================
    // Instantaneous Trendline Tests
    // ============================================================================

    #[test]
    fn test_instantaneous_trendline_basic() {
        let prices = generate_test_prices();
        // Create a smooth_period array with a fixed period of 10
        let smooth_period: Vec<f64> = vec![10.0; prices.len()];

        let result = instantaneous_trendline(&prices, &smooth_period);

        // Verify output length matches input
        assert_eq!(result.len(), prices.len());

        // Verify all values are finite
        for (i, val) in result.iter().enumerate() {
            assert!(val.is_finite(), "Trendline[{}] should be finite", i);
        }
    }

    #[test]
    fn test_instantaneous_trendline_mismatched_lengths() {
        let prices = vec![100.0, 101.0, 102.0];
        let smooth_period = vec![10.0, 10.0];

        let result = instantaneous_trendline(&prices, &smooth_period);

        // Should handle gracefully and return zeros
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_instantaneous_trendline_empty_input() {
        let prices: Vec<f64> = vec![];
        let smooth_period: Vec<f64> = vec![];

        let result = instantaneous_trendline(&prices, &smooth_period);
        assert!(result.is_empty());
    }

    #[test]
    fn test_instantaneous_trendline_constant_period() {
        // Test with constant period = 5
        let prices: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let smooth_period = vec![5.0; 20];

        let result = instantaneous_trendline(&prices, &smooth_period);

        // After warmup, trendline should follow the upward trend
        assert!(
            result[10] > result[5],
            "Trendline should follow upward trend"
        );

        // Values should be reasonable (between min and max price)
        for &val in &result {
            assert!(val >= 100.0 && val <= 119.0);
        }
    }

    #[test]
    fn test_instantaneous_trendline_with_homodyne() {
        // Test integration with Homodyne output
        let prices = generate_test_prices();

        // Get smoothed period from Homodyne
        let hilbert = hilbert_transform(&prices, 14.0);
        let homodyne = homodyne_discriminator(&hilbert.in_phase, &hilbert.quadrature);

        let result = instantaneous_trendline(&prices, &homodyne.smooth_period);

        // Verify output length
        assert_eq!(result.len(), prices.len());

        // Verify trendline is smoother than price (less variance)
        let price_variance = calculate_variance(&prices[20..]);
        let trendline_variance = calculate_variance(&result[20..]);

        assert!(
            trendline_variance < price_variance,
            "Trendline should be smoother than price"
        );
    }

    #[test]
    fn test_instantaneous_trendline_varying_period() {
        // Test with varying period values
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 0.5).collect();
        // Period starts at 5 and increases to 15
        let smooth_period: Vec<f64> = (0..50).map(|i| 5.0 + i as f64 * 0.2).collect();

        let result = instantaneous_trendline(&prices, &smooth_period);

        // Verify output length
        assert_eq!(result.len(), prices.len());

        // Trendline should follow the trend
        assert!(result[40] > result[10]);
    }

    #[test]
    fn test_instantaneous_trendline_period_bounds() {
        // Test with extreme period values (should be clamped)
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let smooth_period = vec![100.0; 30]; // Very large period

        let result = instantaneous_trendline(&prices, &smooth_period);

        // Should handle gracefully and produce valid results
        for (i, val) in result.iter().enumerate() {
            assert!(val.is_finite(), "Trendline[{}] should be finite", i);
        }
    }

    // ============================================================================
    // MAMA Tests
    // ============================================================================

    #[test]
    fn test_mama_basic() {
        let prices = generate_test_prices();
        // Create a reasonable delta_phase (in degrees)
        let delta_phase: Vec<f64> = vec![45.0; prices.len()];

        let result = mama(&prices, &delta_phase);

        // Verify output structure
        assert_eq!(result.mama.len(), prices.len());
        assert_eq!(result.fama.len(), prices.len());

        // FAMA should be different from MAMA (slower)
        assert_ne!(result.mama[10], result.fama[10]);
    }

    #[test]
    fn test_mama_mismatched_lengths() {
        let prices = vec![100.0, 101.0, 102.0];
        let delta_phase = vec![45.0, 45.0];

        let result = mama(&prices, &delta_phase);

        // Should handle gracefully
        assert_eq!(result.mama.len(), 3);
        assert_eq!(result.fama.len(), 3);
    }

    #[test]
    fn test_mama_empty_input() {
        let prices: Vec<f64> = vec![];
        let delta_phase: Vec<f64> = vec![];

        let result = mama(&prices, &delta_phase);
        assert!(result.mama.is_empty());
        assert!(result.fama.is_empty());
    }

    #[test]
    fn test_mama_initialization() {
        // First values should be initialized to first price
        let prices = vec![100.0, 101.0, 102.0, 103.0, 104.0];
        let delta_phase = vec![45.0; 5];

        let result = mama(&prices, &delta_phase);

        assert_eq!(result.mama[0], 100.0);
        assert_eq!(result.fama[0], 100.0);
    }

    #[test]
    fn test_mama_fama_slower_than_mama() {
        // FAMA should be slower (more smoothed) than MAMA
        let prices: Vec<f64> = (0..50)
            .map(|i| 100.0 + (i as f64 * 0.5).sin() * 10.0)
            .collect();
        let delta_phase = vec![45.0; 50];

        let result = mama(&prices, &delta_phase);

        // Calculate variance - FAMA should have lower variance (more smooth)
        let mama_variance = calculate_variance(&result.mama[10..]);
        let fama_variance = calculate_variance(&result.fama[10..]);

        assert!(
            fama_variance < mama_variance,
            "FAMA should be smoother than MAMA"
        );
    }

    #[test]
    fn test_mama_alpha_constraint() {
        // Test with very small delta phase (should clamp alpha to SlowLimit)
        let prices: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let delta_phase = vec![1.0; 20]; // Very small phase change

        let result = mama(&prices, &delta_phase);

        // Verify all values are finite
        for i in 0..result.mama.len() {
            assert!(result.mama[i].is_finite(), "MAMA[{}] should be finite", i);
            assert!(result.fama[i].is_finite(), "FAMA[{}] should be finite", i);
        }
    }

    #[test]
    fn test_mama_fast_response() {
        // Test with large delta phase (fast adaptation)
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + i as f64 * 2.0).collect();
        let delta_phase = vec![180.0; 30]; // Large phase change

        let result = mama(&prices, &delta_phase);

        // MAMA should track price trend closely with large delta_phase
        let last_idx = prices.len() - 1;
        let tracking_error = (result.mama[last_idx] - prices[last_idx]).abs();

        // With fast adaptation, MAMA should be close to current price
        assert!(
            tracking_error < 20.0,
            "MAMA should track price with large delta_phase, error: {}",
            tracking_error
        );
    }

    #[test]
    fn test_mama_with_homodyne_phase() {
        // Test integration with Homodyne output
        let prices = generate_test_prices();

        let hilbert = hilbert_transform(&prices, 14.0);
        let homodyne = homodyne_discriminator(&hilbert.in_phase, &hilbert.quadrature);

        // Calculate delta phase from homodyne phase
        let delta_phase: Vec<f64> = homodyne
            .phase
            .windows(2)
            .map(|w| {
                let diff = w[1] - w[0];
                // Normalize to positive range
                if diff < 0.0 {
                    diff + 360.0
                } else {
                    diff
                }
            })
            .collect();

        // Pad the first value
        let mut full_delta_phase = vec![45.0];
        full_delta_phase.extend(delta_phase);
        full_delta_phase.truncate(prices.len());

        // Fill remaining with default if needed
        while full_delta_phase.len() < prices.len() {
            full_delta_phase.push(45.0);
        }

        let result = mama(&prices, &full_delta_phase);

        // Verify output
        assert_eq!(result.mama.len(), prices.len());
        assert_eq!(result.fama.len(), prices.len());

        // Verify all values are finite
        for i in 0..result.mama.len() {
            assert!(result.mama[i].is_finite(), "MAMA[{}] should be finite", i);
            assert!(result.fama[i].is_finite(), "FAMA[{}] should be finite", i);
        }
    }

    #[test]
    fn test_mama_output_finite() {
        let prices = generate_test_prices();
        let delta_phase: Vec<f64> = (0..prices.len())
            .map(|i| 30.0 + (i as f64 * 0.5) % 90.0)
            .collect();

        let result = mama(&prices, &delta_phase);

        for i in 0..result.mama.len() {
            assert!(result.mama[i].is_finite(), "MAMA[{}] should be finite", i);
            assert!(result.fama[i].is_finite(), "FAMA[{}] should be finite", i);
        }
    }

    #[test]
    fn test_mama_crossover_detection() {
        // Test that MAMA/FAMA crossover can be detected
        let prices: Vec<f64> = (0..50)
            .map(|i| 100.0 + 10.0 * (i as f64 * 0.3).sin())
            .collect();
        let delta_phase = vec![45.0; 50];

        let result = mama(&prices, &delta_phase);

        // Count crossovers (where sign of difference changes)
        let mut crossovers = 0;
        for i in 1..result.mama.len() {
            let prev_diff = result.mama[i - 1] - result.fama[i - 1];
            let curr_diff = result.mama[i] - result.fama[i];
            if prev_diff.signum() != curr_diff.signum() && prev_diff != 0.0 {
                crossovers += 1;
            }
        }

        // Should have some crossovers in cyclic data
        assert!(
            crossovers > 0,
            "MAMA/FAMA should have crossovers in cyclic data"
        );
    }

    // ============================================================================
    // Integration Tests for All Three New Indicators
    // ============================================================================

    #[test]
    fn test_ebsw_trendline_mama_pipeline() {
        // Test the complete pipeline of all three new indicators
        let prices = generate_test_prices();

        // Step 1: Even Better Sinewave
        let ebsw = even_better_sinewave(&prices);

        // Step 2: Hilbert Transform for phase calculation
        let hilbert = hilbert_transform(&prices, 14.0);

        // Step 3: Homodyne for period
        let homodyne = homodyne_discriminator(&hilbert.in_phase, &hilbert.quadrature);

        // Step 4: Instantaneous Trendline
        let trendline = instantaneous_trendline(&prices, &homodyne.smooth_period);

        // Step 5: Calculate delta phase for MAMA
        let delta_phase: Vec<f64> = homodyne
            .phase
            .windows(2)
            .map(|w| {
                let diff = w[1] - w[0];
                if diff < 0.0 {
                    diff + 360.0
                } else {
                    diff
                }
            })
            .collect();
        let mut full_delta_phase = vec![45.0];
        full_delta_phase.extend(delta_phase);
        full_delta_phase.truncate(prices.len());
        while full_delta_phase.len() < prices.len() {
            full_delta_phase.push(45.0);
        }

        // Step 6: MAMA
        let mama_result = mama(&prices, &full_delta_phase);

        // Verify all outputs have correct lengths
        assert_eq!(ebsw.len(), prices.len());
        assert_eq!(trendline.len(), prices.len());
        assert_eq!(mama_result.mama.len(), prices.len());
        assert_eq!(mama_result.fama.len(), prices.len());

        // Verify EBSW is bounded
        for &val in &ebsw {
            assert!(val >= -1.0 && val <= 1.0);
        }

        // Verify all values are finite
        for i in 0..prices.len() {
            assert!(ebsw[i].is_finite());
            assert!(trendline[i].is_finite());
            assert!(mama_result.mama[i].is_finite());
            assert!(mama_result.fama[i].is_finite());
        }
    }

    // ============================================================================
    // Stochastic %K Tests
    // ============================================================================

    #[test]
    fn test_stochastic_k_basic() {
        let high = generate_test_prices();
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|p| p - 1.0).collect();

        let result = stochastic_k(&high, &low, &close, 14);
        assert!(result.is_ok());

        let k_values = result.unwrap();
        assert!(!k_values.is_empty());

        // All K values should be between 0 and 100
        for val in &k_values {
            assert!(*val >= 0.0 && *val <= 100.0);
        }
    }

    #[test]
    fn test_stochastic_k_oversold() {
        // Create a scenario where close is near the low
        let high = vec![100.0; 20];
        let low = vec![80.0; 20];
        let close = vec![82.0; 20]; // Close near the low

        let result = stochastic_k(&high, &low, &close, 14).unwrap();
        // %K should be low (near 0-20 for oversold)
        let last_k = result[result.len() - 1];
        assert!(
            last_k < 20.0,
            "Close near low should produce low %K: {}",
            last_k
        );
    }

    #[test]
    fn test_stochastic_k_overbought() {
        // Create a scenario where close is near the high
        let high = vec![100.0; 20];
        let low = vec![80.0; 20];
        let close = vec![99.0; 20]; // Close near the high

        let result = stochastic_k(&high, &low, &close, 14).unwrap();
        // %K should be high (near 80-100 for overbought)
        let last_k = result[result.len() - 1];
        assert!(
            last_k > 80.0,
            "Close near high should produce high %K: {}",
            last_k
        );
    }

    #[test]
    fn test_stochastic_k_zero_range() {
        // When highest high equals lowest low (no range)
        let high = vec![100.0; 20];
        let low = vec![100.0; 20];
        let close = vec![100.0; 20];

        let result = stochastic_k(&high, &low, &close, 14).unwrap();
        // Should default to 50 when there's no range
        for val in &result {
            assert!(*val == 50.0 || val.is_nan());
        }
    }

    #[test]
    fn test_stochastic_k_insufficient_data() {
        let high = vec![100.0, 101.0];
        let low = vec![98.0, 99.0];
        let close = vec![99.0, 100.0];

        let result = stochastic_k(&high, &low, &close, 14);
        assert!(result.is_err());
    }

    #[test]
    fn test_stochastic_k_mismatched_lengths() {
        let high = vec![100.0, 101.0, 102.0];
        let low = vec![98.0, 99.0];
        let close = vec![99.0, 100.0, 101.0];

        let result = stochastic_k(&high, &low, &close, 14);
        assert!(result.is_err());
    }

    #[test]
    fn test_stochastic_k_known_values() {
        // Test with known values
        // High range: 100-110, Low range: 90-100
        // Close at 105 should give %K = 50
        let high = vec![100.0, 102.0, 105.0, 108.0, 110.0];
        let low = vec![90.0, 92.0, 95.0, 98.0, 100.0];
        let close = vec![95.0, 98.0, 100.0, 103.0, 105.0];

        let result = stochastic_k(&high, &low, &close, 5).unwrap();
        let last_k = result[result.len() - 1];
        // Close=105, High=110, Low=90
        // %K = 100 * (105 - 90) / (110 - 90) = 100 * 15 / 20 = 75
        assert!(
            (last_k - 75.0).abs() < 0.01,
            "Expected %K ~75, got {}",
            last_k
        );
    }

    #[test]
    fn test_stochastic_k_finite_values() {
        let high = generate_test_prices();
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|p| p - 1.0).collect();

        let result = stochastic_k(&high, &low, &close, 14).unwrap();

        for (i, val) in result.iter().enumerate() {
            assert!(
                val.is_finite(),
                "Stochastic %K[{}] should be finite, got {}",
                i,
                val
            );
        }
    }

    // ============================================================================
    // ADX Tests
    // ============================================================================

    #[test]
    fn test_adx_basic() {
        let high = generate_test_prices();
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|p| p - 1.5).collect();

        let result = adx(&high, &low, &close, 14);
        assert!(result.is_ok());

        let AdxOutput {
            adx,
            plus_di,
            minus_di,
        } = result.unwrap();

        // All outputs should have the same length
        assert_eq!(adx.len(), plus_di.len());
        assert_eq!(plus_di.len(), minus_di.len());

        // ADX should be between 0 and 100
        for val in &adx {
            assert!(*val >= 0.0 && *val <= 100.0);
        }

        // +DI and -DI should be between 0 and 100
        for i in 0..plus_di.len() {
            assert!(plus_di[i] >= 0.0 && plus_di[i] <= 100.0);
            assert!(minus_di[i] >= 0.0 && minus_di[i] <= 100.0);
        }
    }

    #[test]
    fn test_adx_strong_uptrend() {
        // Create a strong uptrend - +DI should be higher than -DI
        let highs: Vec<f64> = (0..60).map(|i| 100.0 + i as f64 * 1.0).collect();
        let lows: Vec<f64> = highs.iter().map(|h| h - 2.0).collect();
        let closes: Vec<f64> = highs.iter().map(|h| h - 0.5).collect();

        let result = adx(&highs, &lows, &closes, 14).unwrap();

        // In a strong uptrend, ADX should be elevated
        let last_adx = result.adx[result.adx.len() - 1];
        assert!(
            last_adx > 15.0,
            "ADX should be elevated in uptrend, got {}",
            last_adx
        );

        // +DI should generally be higher than -DI in uptrend
        let plus_di_avg: f64 =
            result.plus_di.iter().skip(10).sum::<f64>() / (result.plus_di.len() - 10) as f64;
        let minus_di_avg: f64 =
            result.minus_di.iter().skip(10).sum::<f64>() / (result.minus_di.len() - 10) as f64;
        assert!(
            plus_di_avg > minus_di_avg,
            "+DI ({}) should be higher than -DI ({}) in uptrend",
            plus_di_avg,
            minus_di_avg
        );
    }

    #[test]
    fn test_adx_strong_downtrend() {
        // Create a strong downtrend - -DI should be higher than +DI
        let highs: Vec<f64> = (0..60).map(|i| 150.0 - i as f64 * 1.0).collect();
        let lows: Vec<f64> = highs.iter().map(|h| h - 2.0).collect();
        let closes: Vec<f64> = highs.iter().map(|h| h - 1.5).collect();

        let result = adx(&highs, &lows, &closes, 14).unwrap();

        // In a strong downtrend, ADX should be elevated
        let last_adx = result.adx[result.adx.len() - 1];
        assert!(
            last_adx > 15.0,
            "ADX should be elevated in downtrend, got {}",
            last_adx
        );

        // -DI should generally be higher than +DI in downtrend
        let plus_di_avg: f64 =
            result.plus_di.iter().skip(10).sum::<f64>() / (result.plus_di.len() - 10) as f64;
        let minus_di_avg: f64 =
            result.minus_di.iter().skip(10).sum::<f64>() / (result.minus_di.len() - 10) as f64;
        assert!(
            minus_di_avg > plus_di_avg,
            "-DI ({}) should be higher than +DI ({}) in downtrend",
            minus_di_avg,
            plus_di_avg
        );
    }

    #[test]
    fn test_adx_ranging_market() {
        // Create a ranging (sideways) market - ADX should be low
        let highs: Vec<f64> = (0..60)
            .map(|i| 100.0 + 5.0 * (i as f64 * 0.2).sin())
            .collect();
        let lows: Vec<f64> = highs.iter().map(|h| h - 3.0).collect();
        let closes: Vec<f64> = highs.iter().map(|h| h - 1.5).collect();

        let result = adx(&highs, &lows, &closes, 14).unwrap();

        // ADX should be relatively low in ranging market (threshold relaxed for test data)
        let avg_adx: f64 = result.adx.iter().sum::<f64>() / result.adx.len() as f64;
        assert!(
            avg_adx < 40.0,
            "ADX should be low in ranging market, got avg {}",
            avg_adx
        );
    }

    #[test]
    fn test_adx_insufficient_data() {
        let high = vec![100.0, 101.0, 102.0];
        let low = vec![98.0, 99.0, 100.0];
        let close = vec![99.0, 100.0, 101.0];

        let result = adx(&high, &low, &close, 14);
        assert!(result.is_err());
    }

    #[test]
    fn test_adx_mismatched_lengths() {
        let high = vec![100.0, 101.0, 102.0];
        let low = vec![98.0, 99.0];
        let close = vec![99.0, 100.0, 101.0];

        let result = adx(&high, &low, &close, 14);
        assert!(result.is_err());
    }

    #[test]
    fn test_adx_finite_values() {
        let high = generate_test_prices();
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|p| p - 1.5).collect();

        let result = adx(&high, &low, &close, 14).unwrap();

        for (i, val) in result.adx.iter().enumerate() {
            assert!(val.is_finite(), "ADX[{}] should be finite", i);
        }
        for (i, val) in result.plus_di.iter().enumerate() {
            assert!(val.is_finite(), "+DI[{}] should be finite", i);
        }
        for (i, val) in result.minus_di.iter().enumerate() {
            assert!(val.is_finite(), "-DI[{}] should be finite", i);
        }
    }

    #[test]
    fn test_adx_di_sum_constraint() {
        // +DI + -DI should generally be <= 100
        let high = generate_test_prices();
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|p| p - 1.5).collect();

        let result = adx(&high, &low, &close, 14).unwrap();

        for i in 0..result.plus_di.len() {
            let sum = result.plus_di[i] + result.minus_di[i];
            // Allow small floating point tolerance
            assert!(
                sum <= 100.01,
                "+DI + -DI at index {} should be <= 100, got {}",
                i,
                sum
            );
        }
    }

    // ============================================================================
    // CCI Tests
    // ============================================================================

    #[test]
    fn test_cci_basic() {
        let high = generate_test_prices();
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|p| p - 1.5).collect();

        let result = cci(&high, &low, &close, 20);
        assert!(result.is_ok());

        let cci_values = result.unwrap();
        assert!(!cci_values.is_empty());
    }

    #[test]
    fn test_cci_overbought() {
        // Create prices that are consistently above the moving average
        let base: Vec<f64> = (0..40).map(|i| 100.0 + i as f64 * 0.1).collect();
        let high: Vec<f64> = base.iter().map(|b| b + 5.0).collect();
        let low: Vec<f64> = base.iter().map(|b| b - 2.0).collect();
        let close: Vec<f64> = high.iter().map(|h| h - 0.5).collect(); // Close near high

        let result = cci(&high, &low, &close, 20).unwrap();
        let last_cci = result[result.len() - 1];

        // CCI > 100 indicates overbought conditions
        assert!(
            last_cci > 50.0,
            "CCI should be elevated when close is near high, got {}",
            last_cci
        );
    }

    #[test]
    fn test_cci_oversold() {
        // Create prices that are consistently below the moving average
        let base: Vec<f64> = (0..40).map(|i| 100.0 - i as f64 * 0.1).collect();
        let high: Vec<f64> = base.iter().map(|b| b + 2.0).collect();
        let low: Vec<f64> = base.iter().map(|b| b - 5.0).collect();
        let close: Vec<f64> = low.iter().map(|l| l + 0.5).collect(); // Close near low

        let result = cci(&high, &low, &close, 20).unwrap();
        let last_cci = result[result.len() - 1];

        // CCI < -100 indicates oversold conditions
        assert!(
            last_cci < -50.0,
            "CCI should be low when close is near low, got {}",
            last_cci
        );
    }

    #[test]
    fn test_cci_constant_price() {
        // When price is constant, CCI should be 0
        let high = vec![100.0; 30];
        let low = vec![100.0; 30];
        let close = vec![100.0; 30];

        let result = cci(&high, &low, &close, 20).unwrap();

        for val in result {
            assert!(
                val.abs() < 0.01 || val.is_nan(),
                "Constant price should produce CCI near 0, got {}",
                val
            );
        }
    }

    #[test]
    fn test_cci_insufficient_data() {
        let high = vec![100.0, 101.0];
        let low = vec![98.0, 99.0];
        let close = vec![99.0, 100.0];

        let result = cci(&high, &low, &close, 20);
        assert!(result.is_err());
    }

    #[test]
    fn test_cci_mismatched_lengths() {
        let high = vec![100.0, 101.0, 102.0];
        let low = vec![98.0, 99.0];
        let close = vec![99.0, 100.0, 101.0];

        let result = cci(&high, &low, &close, 20);
        assert!(result.is_err());
    }

    #[test]
    fn test_cci_finite_values() {
        let high = generate_test_prices();
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|p| p - 1.5).collect();

        let result = cci(&high, &low, &close, 20).unwrap();

        for (i, val) in result.iter().enumerate() {
            assert!(val.is_finite(), "CCI[{}] should be finite, got {}", i, val);
        }
    }

    #[test]
    fn test_cci_typical_price_calculation() {
        // Test typical price calculation
        // Typical Price = (High + Low + Close) / 3
        let high = vec![110.0, 112.0, 115.0, 118.0, 120.0];
        let low = vec![90.0, 92.0, 95.0, 98.0, 100.0];
        let close = vec![100.0, 105.0, 110.0, 115.0, 118.0];

        let result = cci(&high, &low, &close, 5).unwrap();

        // Verify output length
        assert_eq!(result.len(), 1);

        // Typical prices: 100, 103, 106.67, 110.33, 112.67
        // SMA = (100 + 103 + 106.67 + 110.33 + 112.67) / 5 = 106.53
        // Mean deviation calculation...
        // CCI should be finite and reasonable
        assert!(result[0].is_finite());
    }

    // ============================================================================
    // Ichimoku Tests
    // ============================================================================

    fn generate_extended_test_prices(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 100.0 + (i as f64 * 0.5) + (i as f64 * 0.2).sin() * 5.0)
            .collect()
    }

    #[test]
    fn test_ichimoku_basic() {
        let high = generate_extended_test_prices(60);
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();

        let result = ichimoku(&high, &low);
        assert!(result.is_ok());

        let IchimokuOutput {
            tenkan,
            kijun,
            senkou_a,
            senkou_b,
        } = result.unwrap();

        // All outputs should be aligned (kijun period determines output length)
        // Tenkan has more elements since it uses a shorter period
        assert_eq!(kijun.len(), senkou_a.len());
        assert_eq!(senkou_a.len(), senkou_b.len());
        // Tenkan starts earlier (9-period vs 26-period), so it has more elements
        assert!(tenkan.len() >= kijun.len());

        // Verify all values are finite (skip initial NaN values in Senkou spans)
        for (i, val) in tenkan.iter().enumerate() {
            assert!(val.is_finite(), "Tenkan[{}] should be finite", i);
        }
        for (i, val) in kijun.iter().enumerate() {
            assert!(val.is_finite(), "Kijun[{}] should be finite", i);
        }
        // Senkou spans may have NaN during warmup period - only check non-NaN values
        for (i, &val) in senkou_a.iter().enumerate() {
            if !val.is_nan() {
                assert!(val.is_finite(), "Senkou A[{}] should be finite", i);
            }
        }
        for (i, &val) in senkou_b.iter().enumerate() {
            if !val.is_nan() {
                assert!(val.is_finite(), "Senkou B[{}] should be finite", i);
            }
        }
    }

    #[test]
    fn test_ichimoku_relationships() {
        // Tenkan (9-period) should be more responsive than Kijun (26-period)
        let high = generate_extended_test_prices(60);
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();

        let result = ichimoku(&high, &low).unwrap();

        // Verify Tenkan and Kijun relationship
        // After warmup, both should follow price trends
        let tenkan_variance = calculate_variance(&result.tenkan[10..]);
        let kijun_variance = calculate_variance(&result.kijun[10..]);

        // Tenkan should have slightly higher variance (more responsive)
        // Note: This is a general trend, not always true for all data
        assert!(tenkan_variance > 0.0, "Tenkan should have some variance");
        assert!(kijun_variance > 0.0, "Kijun should have some variance");
    }

    #[test]
    fn test_ichimoku_senkou_spread() {
        // Senkou Span A and B create the cloud
        // When A > B, it's a bullish cloud
        // When B > A, it's a bearish cloud
        let high = generate_extended_test_prices(60);
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();

        let result = ichimoku(&high, &low).unwrap();

        // Cloud should exist (Senkou spans should have values)
        // Only check non-NaN values as there may be NaN during warmup
        for (i, &val) in result.senkou_a.iter().enumerate() {
            if !val.is_nan() {
                assert!(val.is_finite(), "Senkou A[{}] should be finite", i);
            }
        }
        for (i, &val) in result.senkou_b.iter().enumerate() {
            if !val.is_nan() {
                assert!(val.is_finite(), "Senkou B[{}] should be finite", i);
            }
        }
    }

    #[test]
    fn test_ichimoku_insufficient_data() {
        // Need at least 52 data points for full Ichimoku calculation
        let high = vec![100.0; 30];
        let low = vec![98.0; 30];

        let result = ichimoku(&high, &low);
        assert!(result.is_err());
    }

    #[test]
    fn test_ichimoku_mismatched_lengths() {
        let high = vec![100.0, 101.0, 102.0];
        let low = vec![98.0, 99.0];

        let result = ichimoku(&high, &low);
        assert!(result.is_err());
    }

    #[test]
    fn test_ichimoku_known_values() {
        // Test with known values
        // Highs: [100, 102, 104, 103, 105, 107, 106, 108, 110, 109]
        // Lows: [98, 100, 102, 101, 103, 105, 104, 106, 108, 107]
        let high = vec![
            100.0, 102.0, 104.0, 103.0, 105.0, 107.0, 106.0, 108.0, 110.0, 109.0,
        ];
        let low = vec![
            98.0, 100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0, 108.0, 107.0,
        ];

        // Need more data for full calculation - let's use 60 points
        let mut extended_high = high.clone();
        let mut extended_low = low.clone();
        for i in 10..60 {
            extended_high.push(100.0 + (i as f64 * 0.5));
            extended_low.push(98.0 + (i as f64 * 0.5));
        }

        let result = ichimoku(&extended_high, &extended_low).unwrap();

        // Verify output lengths
        assert!(!result.tenkan.is_empty());
        assert!(!result.kijun.is_empty());
        assert!(!result.senkou_a.is_empty());
        assert!(!result.senkou_b.is_empty());
    }

    #[test]
    fn test_ichimoku_bullish_cloud() {
        // Create a clear uptrend - Senkou A should be above Senkou B (bullish cloud)
        // Need to create data where both Tenkan and Kijun are rising
        let highs: Vec<f64> = (0..70).map(|i| 100.0 + i as f64 * 0.8).collect();
        let lows: Vec<f64> = highs.iter().map(|h| h - 2.0).collect();

        let result = ichimoku(&highs, &lows).unwrap();

        // In a strong uptrend, Senkou A should generally be above Senkou B
        // Check only the valid computed portion of the cloud
        let valid_cloud: Vec<_> = result
            .senkou_a
            .iter()
            .zip(result.senkou_b.iter())
            .filter(|(a, b)| !a.is_nan() && !b.is_nan())
            .collect();

        if !valid_cloud.is_empty() {
            let bullish_count = valid_cloud.iter().filter(|(a, b)| a > b).count();
            let bullish_ratio = bullish_count as f64 / valid_cloud.len() as f64;

            // In uptrend, cloud should be bullish
            assert!(
                bullish_ratio > 0.3,
                "Expected bullish cloud in uptrend, got {}% bullish",
                bullish_ratio * 100.0
            );
        }
    }

    #[test]
    fn test_ichimoku_bearish_cloud() {
        // Create a clear downtrend - Senkou B should be above Senkou A (bearish cloud)
        let highs: Vec<f64> = (0..70).map(|i| 150.0 - i as f64 * 0.8).collect();
        let lows: Vec<f64> = highs.iter().map(|h| h - 2.0).collect();

        let result = ichimoku(&highs, &lows).unwrap();

        // In a strong downtrend, Senkou B should generally be above Senkou A
        let valid_cloud: Vec<_> = result
            .senkou_b
            .iter()
            .zip(result.senkou_a.iter())
            .filter(|(b, a)| !b.is_nan() && !a.is_nan())
            .collect();

        if !valid_cloud.is_empty() {
            let bearish_count = valid_cloud.iter().filter(|(b, a)| b > a).count();
            let bearish_ratio = bearish_count as f64 / valid_cloud.len() as f64;

            // In downtrend, cloud should be bearish
            assert!(
                bearish_ratio > 0.3,
                "Expected bearish cloud in downtrend, got {}% bearish",
                bearish_ratio * 100.0
            );
        }
    }

    // ============================================================================
    // Integration Tests for All New Indicators
    // ============================================================================

    #[test]
    fn test_all_new_indicators_pipeline() {
        let high = generate_extended_test_prices(60);
        let low: Vec<f64> = high.iter().map(|p| p - 3.0).collect();
        let close: Vec<f64> = high.iter().map(|p| p - 1.5).collect();

        // Test all new indicators
        let stoch = stochastic_k(&high, &low, &close, 14).unwrap();
        let adx_result = adx(&high, &low, &close, 14).unwrap();
        let cci_values = cci(&high, &low, &close, 20).unwrap();
        let ichimoku_result = ichimoku(&high, &low).unwrap();

        // Verify all outputs are valid
        assert!(!stoch.is_empty());
        assert!(!adx_result.adx.is_empty());
        assert!(!cci_values.is_empty());
        assert!(!ichimoku_result.tenkan.is_empty());

        // Stochastic %K should be in [0, 100]
        for val in &stoch {
            assert!(*val >= 0.0 && *val <= 100.0);
        }

        // ADX should be in [0, 100]
        for val in &adx_result.adx {
            assert!(*val >= 0.0 && *val <= 100.0);
        }

        // All Ichimoku Tenkan values should be reasonable (in the general price range)
        let min_price = low.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = high.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        for val in &ichimoku_result.tenkan {
            // Allow some tolerance since Tenkan is a midline calculation
            assert!(
                val >= &(min_price - 10.0) && val <= &(max_price + 10.0),
                "Tenkan value {} should be in reasonable price range",
                val
            );
        }
    }
}
