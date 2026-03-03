//! SuperSignal Module - Multi-Timeframe Phase Confluence Analysis
//!
//! Implements composite signal generation by analyzing multiple timeframes
//! simultaneously, calculating phase alignment bonuses, and detecting
//! inflection points using Ehlers DSP techniques.

use crate::ta_engine::*;
use crate::{MarketData, PhaseConfluenceConfig};
use serde::{Deserialize, Serialize};

/// Market data for a specific timeframe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeframeData {
    /// Timeframe name (e.g., "1m", "5m", "15m", "1h", "4h", "daily")
    pub name: String,
    /// OHLCV market data for this timeframe
    pub ohlcv: Vec<MarketData>,
}

impl TimeframeData {
    /// Extract closing prices from OHLCV data
    pub fn closes(&self) -> Vec<f64> {
        self.ohlcv.iter().map(|d| d.close).collect()
    }

    /// Extract high prices from OHLCV data
    pub fn highs(&self) -> Vec<f64> {
        self.ohlcv.iter().map(|d| d.high).collect()
    }

    /// Extract low prices from OHLCV data
    pub fn lows(&self) -> Vec<f64> {
        self.ohlcv.iter().map(|d| d.low).collect()
    }

    /// Get the latest closing price
    pub fn latest_close(&self) -> Option<f64> {
        self.ohlcv.last().map(|d| d.close)
    }

    /// Get the number of data points
    pub fn len(&self) -> usize {
        self.ohlcv.len()
    }

    /// Check if timeframe has data
    pub fn is_empty(&self) -> bool {
        self.ohlcv.is_empty()
    }
}

/// Technical indicator results for a single timeframe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeframeIndicators {
    /// Relative Strength Index values
    pub rsi: Vec<f64>,
    /// MACD line values
    pub macd_line: Vec<f64>,
    /// MACD signal line values
    pub macd_signal: Vec<f64>,
    /// Bollinger Bands upper band
    pub bollinger_upper: Vec<f64>,
    /// Bollinger Bands lower band
    pub bollinger_lower: Vec<f64>,
    /// Average True Range values
    pub atr: Vec<f64>,
    /// Stochastic %K values
    pub stoch_k: Vec<f64>,
    /// Average Directional Index values
    pub adx: Vec<f64>,
    /// Commodity Channel Index values
    pub cci: Vec<f64>,
    /// Ichimoku Tenkan-sen (Conversion Line)
    pub ichimoku_tenkan: Vec<f64>,
    /// Ichimoku Kijun-sen (Base Line)
    pub ichimoku_kijun: Vec<f64>,
    /// Even Better Sinewave values (Ehlers)
    pub ebsw: Vec<f64>,
    /// Phase values from Homodyne Discriminator (degrees 0-360)
    pub phase: Vec<f64>,
    /// Dominant cycle period values
    pub period: Vec<f64>,
    /// MAMA (Mother of Adaptive Moving Average) values
    pub mama: Vec<f64>,
    /// FAMA (Following Adaptive Moving Average) values
    pub fama: Vec<f64>,
}

impl TimeframeIndicators {
    /// Create empty indicator container
    pub fn new() -> Self {
        Self {
            rsi: Vec::new(),
            macd_line: Vec::new(),
            macd_signal: Vec::new(),
            bollinger_upper: Vec::new(),
            bollinger_lower: Vec::new(),
            atr: Vec::new(),
            stoch_k: Vec::new(),
            adx: Vec::new(),
            cci: Vec::new(),
            ichimoku_tenkan: Vec::new(),
            ichimoku_kijun: Vec::new(),
            ebsw: Vec::new(),
            phase: Vec::new(),
            period: Vec::new(),
            mama: Vec::new(),
            fama: Vec::new(),
        }
    }

    /// Get the latest phase value (most recent)
    pub fn latest_phase(&self) -> Option<f64> {
        self.phase.last().copied()
    }

    /// Get the latest dominant cycle period
    pub fn latest_period(&self) -> Option<f64> {
        self.period.last().copied()
    }

    /// Get the latest EBSW value
    pub fn latest_ebsw(&self) -> Option<f64> {
        self.ebsw.last().copied()
    }
}

impl Default for TimeframeIndicators {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of SuperSignal calculation across all timeframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperSignalResult {
    /// Composite signal score (0-100)
    pub score: f64,
    /// Confidence level (0-1)
    pub confidence: f64,
    /// Phase alignment bonus multiplier (1.0 = no bonus)
    pub phase_alignment: f64,
    /// Indices of detected inflection points in the primary timeframe
    pub inflection_points: Vec<usize>,
    /// Dominant cycle periods per timeframe
    pub dominant_cycles: Vec<f64>,
    /// Per-timeframe contribution to the final score
    pub timeframe_scores: Vec<f64>,
    /// Combined indicator confluence score
    pub indicator_confluence: f64,
}

impl SuperSignalResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            score: 0.0,
            confidence: 0.0,
            phase_alignment: 1.0,
            inflection_points: Vec::new(),
            dominant_cycles: Vec::new(),
            timeframe_scores: Vec::new(),
            indicator_confluence: 0.0,
        }
    }

    /// Check if this is a valid signal (score > threshold)
    pub fn is_valid(&self, threshold: f64) -> bool {
        self.score >= threshold && self.confidence > 0.5
    }

    /// Get signal strength classification
    pub fn strength(&self) -> SignalStrength {
        match self.score {
            s if s >= 80.0 => SignalStrength::Strong,
            s if s >= 60.0 => SignalStrength::Moderate,
            s if s >= 40.0 => SignalStrength::Weak,
            _ => SignalStrength::None,
        }
    }
}

impl Default for SuperSignalResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Signal strength classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalStrength {
    /// No significant signal (score < 40)
    None,
    /// Weak signal (40-60)
    Weak,
    /// Moderate signal (60-80)
    Moderate,
    /// Strong signal (>= 80)
    Strong,
}

/// Calculate all indicators for a single timeframe
pub fn calculate_timeframe_indicators(
    timeframe: &TimeframeData,
    _config: &PhaseConfluenceConfig,
) -> Result<TimeframeIndicators, String> {
    let mut indicators = TimeframeIndicators::new();

    if timeframe.is_empty() {
        return Ok(indicators);
    }

    let closes = timeframe.closes();
    let highs = timeframe.highs();
    let lows = timeframe.lows();

    // RSI calculation
    if let Ok(rsi) = calculate_rsi(&closes, 14) {
        indicators.rsi = rsi;
    }

    // MACD calculation
    if let Ok((macd_line, signal_line, _)) = calculate_macd(&closes, 12, 26, 9) {
        indicators.macd_line = macd_line;
        indicators.macd_signal = signal_line;
    }

    // Bollinger Bands
    if let Ok((upper, _, lower)) = calculate_bollinger(&closes, 20, 2.0) {
        indicators.bollinger_upper = upper;
        indicators.bollinger_lower = lower;
    }

    // ATR
    if let Ok(atr) = calculate_atr(&highs, &lows, &closes, 14) {
        indicators.atr = atr;
    }

    // Stochastic %K
    if let Ok(stoch) = stochastic_k(&highs, &lows, &closes, 14) {
        indicators.stoch_k = stoch;
    }

    // ADX
    if let Ok(adx_output) = adx(&highs, &lows, &closes, 14) {
        indicators.adx = adx_output.adx;
    }

    // CCI
    if let Ok(cci) = cci(&highs, &lows, &closes, 20) {
        indicators.cci = cci;
    }

    // Ichimoku Cloud (Tenkan and Kijun)
    if let Ok(ichi) = ichimoku(&highs, &lows) {
        indicators.ichimoku_tenkan = ichi.tenkan;
        indicators.ichimoku_kijun = ichi.kijun;
    }

    // EBSW (Even Better Sinewave)
    indicators.ebsw = even_better_sinewave(&closes);

    // Hilbert Transform and Homodyne Discriminator for phase/period
    if closes.len() >= 8 {
        let prev_period = 10.0; // Default starting period
        let hilbert = hilbert_transform(&closes, prev_period);
        let homodyne = homodyne_discriminator(&hilbert.in_phase, &hilbert.quadrature);

        // Calculate MAMA/FAMA using phase changes before moving
        if homodyne.phase.len() >= 2 {
            let delta_phase: Vec<f64> = homodyne
                .phase
                .windows(2)
                .map(|w| {
                    let diff = w[1] - w[0];
                    // Handle phase wrap-around
                    if diff > 180.0 {
                        diff - 360.0
                    } else if diff < -180.0 {
                        diff + 360.0
                    } else {
                        diff
                    }
                })
                .collect();

            // Pad delta_phase to match price length
            let mut full_delta_phase = vec![0.0; closes.len() - delta_phase.len()];
            full_delta_phase.extend(delta_phase);

            let mama_output = mama(&closes, &full_delta_phase);
            indicators.mama = mama_output.mama;
            indicators.fama = mama_output.fama;
        }

        // Assign phase and period after MAMA calculation
        indicators.phase = homodyne.phase;
        indicators.period = homodyne.period;
    }

    Ok(indicators)
}

/// Calculate phase alignment bonus
/// When ≥3 timeframes have phase within ±15° → apply constructive interference multiplier
/// Formula: bonus = 1 + (n_aligned - 2) * 0.25 where n_aligned ≥ 3
pub fn calculate_phase_alignment_bonus(
    indicators: &[TimeframeIndicators],
    tolerance_degrees: f64,
) -> f64 {
    if indicators.len() < 3 {
        return 1.0; // Not enough timeframes for alignment bonus
    }

    // Get latest phase from each timeframe
    let phases: Vec<f64> = indicators
        .iter()
        .filter_map(|ind| ind.latest_phase())
        .collect();

    if phases.len() < 3 {
        return 1.0;
    }

    // Find the maximum number of aligned phases
    let mut max_aligned = 0;

    for i in 0..phases.len() {
        let reference_phase = phases[i];
        let mut aligned_count = 1; // Count the reference phase

        for (j, &phase) in phases.iter().enumerate() {
            if i == j {
                continue;
            }

            // Calculate angular distance (handle wrap-around)
            let mut diff = (phase - reference_phase).abs();
            if diff > 180.0 {
                diff = 360.0 - diff;
            }

            if diff <= tolerance_degrees {
                aligned_count += 1;
            }
        }

        max_aligned = max_aligned.max(aligned_count);
    }

    // Calculate bonus: 1 + (n_aligned - 2) * 0.25 for n_aligned >= 3
    if max_aligned >= 3 {
        1.0 + (max_aligned - 2) as f64 * 0.25
    } else {
        1.0
    }
}

/// Detect inflection points using EBSW zero crossings and phase transitions
pub fn detect_inflection_points(
    indicators: &TimeframeIndicators,
    timeframe_data: &TimeframeData,
) -> Vec<usize> {
    let mut inflections = Vec::new();

    // Method 1: EBSW zero crossings
    if indicators.ebsw.len() >= 2 {
        for i in 1..indicators.ebsw.len() {
            let prev = indicators.ebsw[i - 1];
            let curr = indicators.ebsw[i];

            // Detect zero crossing
            if (prev < 0.0 && curr >= 0.0) || (prev > 0.0 && curr <= 0.0) {
                inflections.push(i);
            }
        }
    }

    // Method 2: Phase transition through 0° or 180° (cycle peaks/troughs)
    if indicators.phase.len() >= 2 {
        for i in 1..indicators.phase.len() {
            let prev = indicators.phase[i - 1];
            let curr = indicators.phase[i];

            // Detect phase crossing 0° or 180° (with tolerance)
            const TOLERANCE: f64 = 15.0;

            // Crossing 0° (or 360°)
            if (prev > 360.0 - TOLERANCE && curr < TOLERANCE)
                || (prev < TOLERANCE && curr > 360.0 - TOLERANCE)
            {
                if !inflections.contains(&i) {
                    inflections.push(i);
                }
            }

            // Crossing 180° (inversion point)
            if (prev < 180.0 + TOLERANCE && curr > 180.0 - TOLERANCE)
                || (prev > 180.0 - TOLERANCE && curr < 180.0 + TOLERANCE)
            {
                if !inflections.contains(&i) {
                    inflections.push(i);
                }
            }
        }
    }

    // Method 3: Price divergence from MAMA/FAMA
    if indicators.mama.len() >= 2 && indicators.fama.len() >= 2 {
        let closes = timeframe_data.closes();
        let offset = closes.len().saturating_sub(indicators.mama.len());

        for i in 1..indicators.mama.len() {
            let mama_cross = (indicators.mama[i - 1] <= indicators.fama[i - 1]
                && indicators.mama[i] > indicators.fama[i])
                || (indicators.mama[i - 1] >= indicators.fama[i - 1]
                    && indicators.mama[i] < indicators.fama[i]);

            if mama_cross {
                let price_idx = i + offset;
                if price_idx < closes.len() && !inflections.contains(&price_idx) {
                    inflections.push(price_idx);
                }
            }
        }
    }

    // Sort and return
    inflections.sort_unstable();
    inflections
}

/// Calculate individual indicator confluence score for a timeframe
fn calculate_indicator_confluence(indicators: &TimeframeIndicators, price: f64) -> f64 {
    let mut signals = Vec::new();

    // RSI signal (oversold < 30, overbought > 70)
    if let Some(&rsi) = indicators.rsi.last() {
        if rsi < 30.0 {
            signals.push(1.0); // Bullish
        } else if rsi > 70.0 {
            signals.push(-1.0); // Bearish
        } else {
            signals.push(0.0);
        }
    }

    // MACD signal
    if let (Some(&macd), Some(&signal)) =
        (indicators.macd_line.last(), indicators.macd_signal.last())
    {
        if macd > signal {
            signals.push(1.0);
        } else {
            signals.push(-1.0);
        }
    }

    // Bollinger Bands signal
    if let (Some(&upper), Some(&lower)) = (
        indicators.bollinger_upper.last(),
        indicators.bollinger_lower.last(),
    ) {
        if price < lower {
            signals.push(1.0); // Oversold
        } else if price > upper {
            signals.push(-1.0); // Overbought
        } else {
            signals.push(0.0);
        }
    }

    // EBSW signal
    if let Some(&ebsw) = indicators.ebsw.last() {
        if ebsw > 0.5 {
            signals.push(1.0);
        } else if ebsw < -0.5 {
            signals.push(-1.0);
        } else {
            signals.push(0.0);
        }
    }

    // Stochastic signal
    if let Some(&stoch) = indicators.stoch_k.last() {
        if stoch < 20.0 {
            signals.push(1.0);
        } else if stoch > 80.0 {
            signals.push(-1.0);
        } else {
            signals.push(0.0);
        }
    }

    // Calculate confluence (agreement among indicators)
    if signals.is_empty() {
        return 0.0;
    }

    let sum: f64 = signals.iter().sum();
    let avg = sum / signals.len() as f64;

    // Normalize to 0-1 range (absolute value shows strength regardless of direction)
    avg.abs()
}

/// Calculate composite SuperSignal across all timeframes
pub fn calculate_super_signal(
    timeframes: &[TimeframeData],
    config: &PhaseConfluenceConfig,
) -> SuperSignalResult {
    let mut result = SuperSignalResult::new();

    if timeframes.is_empty() {
        return result;
    }

    // Calculate indicators for each timeframe
    let mut all_indicators: Vec<TimeframeIndicators> = Vec::with_capacity(timeframes.len());
    let mut timeframe_scores: Vec<f64> = Vec::with_capacity(timeframes.len());

    for tf in timeframes {
        match calculate_timeframe_indicators(tf, config) {
            Ok(indicators) => {
                // Calculate confluence score for this timeframe
                let price = tf.latest_close().unwrap_or(0.0);
                let confluence = calculate_indicator_confluence(&indicators, price);

                timeframe_scores.push(confluence);
                all_indicators.push(indicators);
            }
            Err(_) => {
                timeframe_scores.push(0.0);
                all_indicators.push(TimeframeIndicators::new());
            }
        }
    }

    // Calculate phase alignment bonus (±15° tolerance)
    const PHASE_TOLERANCE: f64 = 15.0;
    let phase_bonus = calculate_phase_alignment_bonus(&all_indicators, PHASE_TOLERANCE);
    result.phase_alignment = phase_bonus;

    // Calculate dominant cycles per timeframe
    result.dominant_cycles = all_indicators
        .iter()
        .filter_map(|ind| ind.latest_period())
        .collect();

    // Calculate weighted composite score
    // Higher timeframes get more weight
    let weights: Vec<f64> = (0..timeframes.len())
        .map(|i| 1.0 + i as f64 * 0.5)
        .collect();

    let total_weight: f64 = weights.iter().sum();
    let weighted_sum: f64 = timeframe_scores
        .iter()
        .zip(weights.iter())
        .map(|(score, weight)| score * weight)
        .sum();

    let base_score = if total_weight > 0.0 {
        (weighted_sum / total_weight) * 100.0
    } else {
        0.0
    };

    // Apply phase alignment bonus
    result.score = (base_score * phase_bonus).min(100.0);
    result.timeframe_scores = timeframe_scores.clone();

    // Calculate confidence based on data quality and agreement
    let valid_timeframes = timeframe_scores.iter().filter(|&&s| s > 0.0).count();
    let agreement_variance = calculate_variance(&timeframe_scores);
    let confidence_base = (valid_timeframes as f64 / timeframes.len() as f64).min(1.0);
    let agreement_factor = (1.0 - agreement_variance.min(1.0)).max(0.0);

    result.confidence = (confidence_base * 0.5 + agreement_factor * 0.5).min(1.0);
    result.indicator_confluence = weighted_sum / total_weight;

    // Detect inflection points in the primary (highest) timeframe
    if let Some(primary_tf) = timeframes.last() {
        if let Some(primary_ind) = all_indicators.last() {
            result.inflection_points = detect_inflection_points(primary_ind, primary_tf);
        }
    }

    result
}

/// Calculate variance of a slice of f64 values
fn calculate_variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

    variance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_market_data(n: usize) -> Vec<MarketData> {
        (0..n)
            .map(|i| {
                let price = 100.0 + (i as f64 * 0.5) + (i as f64 * 0.1).sin() * 5.0;
                MarketData {
                    timestamp: 1704295800000 + (i as u64 * 60000),
                    open: price - 0.5,
                    high: price + 1.0,
                    low: price - 1.0,
                    close: price,
                    volume: 1000000 + (i as u64 * 1000),
                }
            })
            .collect()
    }

    #[test]
    fn test_timeframe_data_extraction() {
        let ohlcv = create_test_market_data(20);
        let tf = TimeframeData {
            name: "1m".to_string(),
            ohlcv: ohlcv.clone(),
        };

        assert_eq!(tf.closes().len(), 20);
        assert_eq!(tf.len(), 20);
        assert!(!tf.is_empty());
    }

    #[test]
    fn test_phase_alignment_bonus() {
        // Create indicators with aligned phases (all at 45°)
        let aligned_indicators: Vec<TimeframeIndicators> = (0..4)
            .map(|_| TimeframeIndicators {
                phase: vec![0.0, 45.0],
                ..TimeframeIndicators::new()
            })
            .collect();

        let bonus = calculate_phase_alignment_bonus(&aligned_indicators, 15.0);
        assert!(bonus > 1.0, "Aligned phases should give bonus > 1.0");
        assert_eq!(bonus, 1.5, "4 aligned timeframes should give 1.5 bonus");

        // Create indicators with scattered phases
        let scattered: Vec<TimeframeIndicators> = vec![
            TimeframeIndicators {
                phase: vec![0.0, 0.0],
                ..TimeframeIndicators::new()
            },
            TimeframeIndicators {
                phase: vec![0.0, 90.0],
                ..TimeframeIndicators::new()
            },
            TimeframeIndicators {
                phase: vec![0.0, 180.0],
                ..TimeframeIndicators::new()
            },
        ];

        let no_bonus = calculate_phase_alignment_bonus(&scattered, 15.0);
        assert_eq!(no_bonus, 1.0, "Scattered phases should give no bonus");
    }

    #[test]
    fn test_super_signal_calculation() {
        let timeframes = vec![
            TimeframeData {
                name: "5m".to_string(),
                ohlcv: create_test_market_data(100),
            },
            TimeframeData {
                name: "15m".to_string(),
                ohlcv: create_test_market_data(100),
            },
            TimeframeData {
                name: "1h".to_string(),
                ohlcv: create_test_market_data(100),
            },
        ];

        let config = PhaseConfluenceConfig::default();
        let result = calculate_super_signal(&timeframes, &config);

        assert!(result.score >= 0.0 && result.score <= 100.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
        assert!(!result.dominant_cycles.is_empty());
    }

    #[test]
    fn test_inflection_point_detection() {
        // Create EBSW with known zero crossings
        let indicators = TimeframeIndicators {
            ebsw: vec![-0.5, -0.2, 0.1, 0.5, 0.3, -0.1, -0.4],
            ..TimeframeIndicators::new()
        };

        let tf_data = TimeframeData {
            name: "test".to_string(),
            ohlcv: create_test_market_data(7),
        };

        let points = detect_inflection_points(&indicators, &tf_data);
        assert!(!points.is_empty(), "Should detect zero crossings");
        assert!(points.contains(&2), "Should detect crossing at index 2");
        assert!(points.contains(&5), "Should detect crossing at index 5");
    }

    #[test]
    fn test_signal_strength_classification() {
        let strong = SuperSignalResult {
            score: 85.0,
            ..SuperSignalResult::new()
        };
        assert_eq!(strong.strength(), SignalStrength::Strong);

        let weak = SuperSignalResult {
            score: 45.0,
            ..SuperSignalResult::new()
        };
        assert_eq!(weak.strength(), SignalStrength::Weak);

        let none = SuperSignalResult {
            score: 20.0,
            ..SuperSignalResult::new()
        };
        assert_eq!(none.strength(), SignalStrength::None);
    }
}
