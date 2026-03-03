# PhaseConfluence SuperSignal WASM Skill

A high-performance WebAssembly module implementing a multi-timeframe phasing super-signal for detecting market inflection points with rigorous robustness validation.

## Overview

The PhaseConfluence strategy combines **Ehlers Digital Signal Processing (DSP)** techniques with traditional technical analysis to identify high-probability trading opportunities. By analyzing multiple timeframes simultaneously and detecting phase alignment across market cycles, the system generates composite signals with enhanced confidence metrics.

### Key Features

- **Multi-timeframe analysis**: Simultaneous processing of 1m, 5m, 15m, 1h, 4h, and daily timeframes
- **Phase confluence detection**: Identifies when multiple timeframes align in cycle phase
- **Inflection point detection**: Pinpoints potential market turning points using DSP techniques
- **Robustness gates**: Monte-Carlo simulation, Walk-Forward Efficiency, and Profit Factor validation
- **WASM-native performance**: Zero-overhead execution in browser or server environments

## Theoretical Foundation

### Ehlers Hilbert Transform Phasing Theory

The PhaseConfluence strategy is built on John F. Ehlers' revolutionary work in applying digital signal processing to financial markets:

> *"The markets are not random. They have a structure that can be measured and predicted using the tools of digital signal processing."* — John F. Ehlers

**Core Concepts:**

1. **Hilbert Transform Phasing**: The Hilbert Transform creates a complex signal from real price data, enabling the extraction of instantaneous phase and amplitude information. This allows the algorithm to measure where in its cycle the market is at any given moment.

2. **Dominant Cycle Extraction**: Using the Homodyne Discriminator, the system measures the dominant cycle period in real-time. Unlike fixed-period indicators, this adapts to changing market conditions.

3. **Phase Alignment / Constructive Interference**: When multiple timeframes align in phase (0°, 90°, 180°, 270°), the probability of a significant price move increases. The system calculates a "phase alignment bonus" that amplifies signals when constructive interference occurs across timeframes.

### References

This implementation draws heavily from:

- **"Rocket Science for Traders: Digital Signal Processing Applications"** (2001) — John F. Ehlers  
  Covers the foundation of DSP in trading, including the Hilbert Transform, SuperSmoother, and Roofing Filter.

- **"Cycle Analytics for Traders"** (2013) — John F. Ehlers  
  Advanced cycle measurement techniques, MAMA/FAMA adaptive moving averages, and the Even Better Sinewave indicator.

## Indicators Implemented

### Ehlers DSP Indicators

| Indicator | Description | Reference |
|-----------|-------------|-----------|
| **SuperSmoother** | Two-pole IIR filter with critical period 10 bars; eliminates aliasing noise | *Rocket Science for Traders*, Ch. 3 |
| **Roofing Filter** | High-pass + SuperSmoother cascade; isolates cyclic components 10-48 bars | *Rocket Science for Traders*, Ch. 4 |
| **Hilbert Transform** | Generates in-phase (I) and quadrature (Q) components for phase analysis | *Rocket Science for Traders*, Ch. 5 |
| **Homodyne Discriminator** | Measures dominant cycle period using the product method | *Rocket Science for Traders*, Ch. 6 |
| **Even Better Sinewave (EBSW)** | Normalized cyclic indicator bounded [-1, +1] with trend removal | *Cycle Analytics for Traders*, Ch. 4 |
| **Instantaneous Trendline** | Adaptive trend following based on measured cycle period | *Rocket Science for Traders*, Ch. 7 |
| **MAMA/FAMA** | Mother of Adaptive Moving Average & Following Adaptive Moving Average; adaptive smoothing based on phase rate of change | *Rocket Science for Traders*, Ch. 8 |

### Standard Technical Indicators

| Indicator | Period | Usage |
|-----------|--------|-------|
| **RSI** (Relative Strength Index) | 14 | Momentum oscillator for overbought/oversold |
| **MACD** | 12/26/9 | Trend-following momentum indicator |
| **Bollinger Bands** | 20 (2σ) | Volatility-based support/resistance |
| **ATR** (Average True Range) | 14 | Volatility measurement for stop placement |
| **Stochastic %K** | 14 | Momentum indicator comparing close to range |
| **ADX** (Average Directional Index) | 14 | Trend strength measurement |
| **CCI** (Commodity Channel Index) | 20 | Cyclic momentum indicator |
| **Ichimoku Cloud** | 9/26 | Trend direction and support/resistance levels |

## Usage

### JSON Input Format

The [`process_command()`](src/lib.rs:373) function accepts JSON input with multi-timeframe OHLCV data and optional configuration:

```json
{
  "timeframes": [
    {
      "name": "1h",
      "ohlcv": [
        {
          "timestamp": 1704067200000,
          "open": 42000.0,
          "high": 42500.0,
          "low": 41800.0,
          "close": 42300.0,
          "volume": 1500000
        }
      ]
    },
    {
      "name": "4h",
      "ohlcv": [ ... ]
    },
    {
      "name": "daily",
      "ohlcv": [ ... ]
    }
  ],
  "config": {
    "momentum_period_fast": 12,
    "momentum_period_slow": 26,
    "trend_period": 50,
    "volatility_period": 14,
    "minimum_signal_score": 70.0,
    "maximum_atr_risk": 0.02
  },
  "trades": [
    {
      "entry_price": 42000.0,
      "exit_price": 42500.0,
      "quantity": 1.0,
      "entry_time": 1704067200000,
      "exit_time": 1704153600000
    }
  ]
}
```

### JSON Output Format

The function returns a [`ProcessCommandOutput`](src/lib.rs:208) structure:

```json
{
  "super_signal_score": 85.5,
  "confidence": 0.92,
  "inflection_points": [42, 78, 134],
  "dominant_cycles": [18.3, 22.7, 35.1],
  "suggested_stops": {
    "long_stop": 41850.5,
    "short_stop": 42875.0,
    "atr_multiplier": 2.0
  },
  "robustness_metrics": {
    "monte_carlo_dd_99": 0.12,
    "walk_forward_efficiency": 0.68,
    "profit_factor": 1.85,
    "gates_passed": true
  },
  "phase_alignment": 1.35,
  "timeframe_scores": [75.0, 88.5, 92.0],
  "indicator_confluence": 0.87
}
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `super_signal_score` | f64 | Composite signal strength (0-100) |
| `confidence` | f64 | Statistical confidence (0.0-1.0) |
| `inflection_points` | [usize] | Indices of detected market turning points |
| `dominant_cycles` | [f64] | Measured cycle periods per timeframe |
| `suggested_stops` | object | ATR-based stop loss recommendations |
| `robustness_metrics` | object | Validation metrics (see Robustness Gates) |
| `phase_alignment` | f64 | Multi-timeframe phase alignment multiplier |
| `timeframe_scores` | [f64] | Individual timeframe contributions |
| `indicator_confluence` | f64 | Agreement level across standard indicators |

### WASM Integration Example

```javascript
import init, { process_command_wasm } from './phase_confluence.js';

async function analyzeMarket(timeframeData) {
  await init();
  
  const input = {
    timeframes: timeframeData,
    config: {
      momentum_period_fast: 12,
      momentum_period_slow: 26,
      trend_period: 50,
      volatility_period: 14
    }
  };
  
  const result = process_command_wasm(JSON.stringify(input));
  const signal = JSON.parse(result);
  
  if (signal.robustness_metrics.gates_passed && signal.super_signal_score > 75) {
    console.log(`Strong signal: ${signal.super_signal_score}/100`);
    console.log(`Confidence: ${(signal.confidence * 100).toFixed(1)}%`);
  }
  
  return signal;
}
```

## WASM Build

### Prerequisites

```bash
# Install Rust target for WASM
cargo install wasm-bindgen-cli
rustup target add wasm32-wasip1
```

### Build Commands

```bash
# Build release version
cargo build --target wasm32-wasip1 --release

# Output path
target/wasm32-wasip1/release/phase_confluence.wasm
```

### Build Optimization

The [`Cargo.toml`](Cargo.toml) includes optimization settings for minimal WASM size:

```toml
[profile.release]
opt-level = 3
lto = true
panic = "abort"
```

### Web Integration

The build script [`build.rs`](build.rs) generates JavaScript bindings:

```bash
# Generate wasm-bindgen bindings
wasm-bindgen target/wasm32-wasip1/release/phase_confluence.wasm \
  --out-dir ./pkg \
  --target web
```

## Robustness Gates

The PhaseConfluence strategy enforces strict robustness criteria before signals are considered valid:

### Monte Carlo Simulation (5000 iterations, seed 42)

Performs bootstrap resampling of historical trade returns to estimate the 99th percentile maximum drawdown:

```rust
pub fn monte_carlo_dd_99(returns: &[f64], seed: u64) -> f64
```

- **Iterations**: 5000
- **Seed**: 42 (deterministic)
- **Output**: 99th percentile drawdown (e.g., 0.15 = 15% max drawdown)

### Walk-Forward Efficiency (WFE)

Measures consistency between in-sample (IS) and out-of-sample (OS) performance:

```
WFE = Sharpe_IS / Sharpe_OS
```

- **Minimum**: 0.5 (50% efficiency)
- **Rationale**: Strategy must perform consistently across different market regimes

### Profit Factor

Ratio of gross profit to gross loss:

```
Profit Factor = Σ(Gross Profits) / Σ(|Gross Losses|)
```

- **Minimum**: 1.5
- **Rationale**: Each dollar risked should return at least $1.50

### Gate Logic

All three gates must pass for `gates_passed` to be `true`:

```rust
let gates_passed = mc_dd < 0.20 && wfe > 0.5 && pf > 1.5;
```

| Gate | Threshold | Purpose |
|------|-----------|---------|
| Monte Carlo DD₉₉ | < 20% | Risk of ruin validation |
| Walk-Forward Efficiency | > 0.5 | Consistency across regimes |
| Profit Factor | > 1.5 | Positive expectancy |

## License

Licensed under either of:

- **MIT License** — See [LICENSE-MIT](../../LICENSE-MIT)
- **Apache License, Version 2.0** — See [LICENSE-APACHE](../../LICENSE-APACHE)

at your option.

## Contributing

Contributions are welcome! Please ensure:

1. All indicators include corresponding unit tests
2. Robustness gates pass on historical backtests
3. Code follows the existing Rust style
4. Documentation is updated for new features

## References

1. Ehlers, J. F. (2001). *Rocket Science for Traders: Digital Signal Processing Applications*. Wiley.
2. Ehlers, J. F. (2013). *Cycle Analytics for Traders*. Wiley.
3. Ehlers, J. F. (2004). *Cybernetic Analysis for Stocks and Futures*. Wiley.

---

*Built with Rust 🦀 and DSP precision for quantitative trading systems.*
