//! In-process deterministic `Theta` (`theta_calc`) service wrapper.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use theta_calc::{
    AnalyzeGraphRequest, AnalyzeGraphResponse, EvalRequest, EvalResultPayload, FormulaRequest,
    FormulaResponse, SolveRequest, SolveResponse, StatsRequest, StatsResponse, TransformRequest,
    TransformResponse, calc_analyze_graph, calc_eval, calc_formula, calc_solve, calc_stats,
    calc_transform, dispatch_tool_json,
};

use crate::error::{HydrogenError, Result};

/// Stage-matched defaults applied automatically to `theta_calc` calls when not explicitly overridden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagePresetConfig {
    pub preset: String,
    pub angle_mode: String,
    pub allow_mixed_fractions: bool,
    pub anchor_tokens: bool,
}

impl StagePresetConfig {
    /// Default configuration for `Gyara+` 11+ Mathematics (`rules/gyaraplus/maths/common.md`).
    pub fn gyara_11plus_maths() -> Self {
        Self {
            preset: "uk_gcse".to_string(),
            angle_mode: "deg".to_string(),
            allow_mixed_fractions: true,
            anchor_tokens: true,
        }
    }

    /// Default configuration for UK GCSE Mathematics (`rules/gcse/maths/common.md`).
    pub fn gcse_maths() -> Self {
        Self {
            preset: "uk_gcse".to_string(),
            angle_mode: "deg".to_string(),
            allow_mixed_fractions: false,
            anchor_tokens: true,
        }
    }
}

impl Default for StagePresetConfig {
    fn default() -> Self {
        Self::gyara_11plus_maths()
    }
}

/// Stateless in-process `Theta` wrapper (`< 50 µs` deterministic evaluation & verification).
#[derive(Debug, Clone, Default)]
pub struct ThetaService {
    stage: StagePresetConfig,
}

impl ThetaService {
    /// Construct a `ThetaService` bound to the given curriculum stage preset.
    pub fn new(stage: StagePresetConfig) -> Self {
        Self { stage }
    }

    /// Active stage preset configuration.
    pub fn stage(&self) -> &StagePresetConfig {
        &self.stage
    }

    /// Evaluate a mathematical expression (`calc.eval`) using stage defaults for omitted options.
    pub fn eval_expr(&self, expr: &str) -> Result<EvalResultPayload> {
        self.eval_with_vars(expr, BTreeMap::new())
    }

    /// Evaluate an expression with variable bindings (`vars`).
    pub fn eval_with_vars(
        &self,
        expr: &str,
        vars: BTreeMap<String, Value>,
    ) -> Result<EvalResultPayload> {
        let req = EvalRequest {
            expr: expr.to_string(),
            vars,
            preset: Some(self.stage.preset.clone()),
            angle_mode: Some(self.stage.angle_mode.clone()),
            space: None,
            allow_mixed_fractions: Some(self.stage.allow_mixed_fractions),
            anchor_tokens: Some(self.stage.anchor_tokens),
        };
        self.eval(&req)
    }

    /// Execute a full `EvalRequest`, filling stage defaults where fields are `None`.
    pub fn eval(&self, req: &EvalRequest) -> Result<EvalResultPayload> {
        let mut merged = req.clone();
        if merged.preset.is_none() {
            merged.preset = Some(self.stage.preset.clone());
        }
        if merged.angle_mode.is_none() {
            merged.angle_mode = Some(self.stage.angle_mode.clone());
        }
        let allow_mixed = merged
            .allow_mixed_fractions
            .unwrap_or(self.stage.allow_mixed_fractions);
        merged.allow_mixed_fractions = Some(allow_mixed);
        if merged.anchor_tokens.is_none() {
            merged.anchor_tokens = Some(self.stage.anchor_tokens);
        }
        if allow_mixed && !merged.expr.contains('\\') {
            merged.expr = normalize_ascii_mixed_fractions(&merged.expr);
        }

        let resp = calc_eval(&merged);
        if let Some(res) = resp.result
            && resp.ok
        {
            return Ok(res);
        }
        let msg = resp
            .error
            .map(|e| format!("{e:?}"))
            .unwrap_or_else(|| "unknown calc.eval failure".to_string());
        Err(HydrogenError::Theta(msg))
    }

    /// Solve one or more equations (`calc.solve`).
    pub fn solve(&self, req: &SolveRequest) -> Result<SolveResponse> {
        let mut merged = req.clone();
        if merged.preset.is_none() {
            merged.preset = Some(self.stage.preset.clone());
        }
        if merged.angle_mode.is_none() {
            merged.angle_mode = Some(self.stage.angle_mode.clone());
        }
        if merged.anchor_tokens.is_none() {
            merged.anchor_tokens = Some(self.stage.anchor_tokens);
        }
        let resp = calc_solve(&merged);
        if resp.ok {
            Ok(resp)
        } else {
            let msg = resp
                .error
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "unknown calc.solve failure".to_string());
            Err(HydrogenError::Theta(msg))
        }
    }

    /// Convenience helper for solving simultaneous or single equations with optional step trace.
    pub fn solve_equations(
        &self,
        equations: &[&str],
        solve_for: &[&str],
        trace: bool,
    ) -> Result<SolveResponse> {
        let req = SolveRequest {
            equations: equations.iter().map(|s| (*s).to_string()).collect(),
            solve_for: solve_for.iter().map(|s| (*s).to_string()).collect(),
            trace,
            preset: Some(self.stage.preset.clone()),
            angle_mode: Some(self.stage.angle_mode.clone()),
            anchor_tokens: Some(self.stage.anchor_tokens),
            ..SolveRequest::default()
        };
        self.solve(&req)
    }

    /// Execute a physics/maths formula lookup & substitution (`calc.formula`).
    pub fn formula(&self, req: &FormulaRequest) -> Result<FormulaResponse> {
        let resp = calc_formula(req);
        if resp.ok {
            Ok(resp)
        } else {
            let msg = resp
                .error
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "unknown calc.formula failure".to_string());
            Err(HydrogenError::Theta(msg))
        }
    }

    /// Execute symbolic transformation (`mixed_fraction`, `factor`, `expand`, `simplify`, etc.).
    pub fn transform(&self, req: &TransformRequest) -> Result<TransformResponse> {
        let resp = calc_transform(req);
        if resp.ok {
            Ok(resp)
        } else {
            let msg = resp
                .error
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "unknown calc.transform failure".to_string());
            Err(HydrogenError::Theta(msg))
        }
    }

    /// Compute exact descriptive statistics / regression / distributions (`calc.stats`).
    pub fn stats(&self, req: &StatsRequest) -> Result<StatsResponse> {
        let resp = calc_stats(req);
        if resp.ok {
            Ok(resp)
        } else {
            let msg = resp
                .error
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "unknown calc.stats failure".to_string());
            Err(HydrogenError::Theta(msg))
        }
    }

    /// Analyze a 2D curve (`roots`, `y_intercept`, `stationary_points`, `asymptotes` — `calc.analyze_graph`).
    pub fn analyze_graph(&self, req: &AnalyzeGraphRequest) -> Result<AnalyzeGraphResponse> {
        let resp = calc_analyze_graph(req);
        if resp.ok {
            Ok(resp)
        } else {
            let msg = resp
                .error
                .map(|e| format!("{e:?}"))
                .unwrap_or_else(|| "unknown calc.analyze_graph failure".to_string());
            Err(HydrogenError::Theta(msg))
        }
    }

    /// Dispatch a JSON tool call (`calc.eval`, `calc.solve`, etc.) directly for LLM tool-calling loops.
    pub fn dispatch_tool(&self, tool_name: &str, args: &Value) -> Result<Value> {
        let raw_out = dispatch_tool_json(tool_name, &args.to_string());
        let val: Value = serde_json::from_str(&raw_out)?;
        Ok(val)
    }

    /// Deterministically check whether `lhs` and `rhs` evaluate to the exact same value (`(lhs) - (rhs) == 0`).
    pub fn expressions_equivalent(&self, lhs: &str, rhs: &str) -> Result<bool> {
        let diff_expr = format!("({lhs}) - ({rhs})");
        let diff = self.eval_expr(&diff_expr)?;
        Ok(diff.exact == "0")
    }

    /// Verify 5-option (`A–E`) MCQ mathematical uniqueness:
    /// 1. Every option parses & evaluates cleanly,
    /// 2. All options are pairwise distinct,
    /// 3. Exactly one option equals `expected_exact` (returning its 0-based index).
    pub fn verify_mcq_options(&self, options: &[&str], expected_exact: &str) -> Result<usize> {
        if options.len() < 2 {
            return Err(HydrogenError::Validation(
                "MCQ verification requires at least 2 options".into(),
            ));
        }

        let mut canonical_values: Vec<String> = Vec::with_capacity(options.len());
        let mut matching_indices = Vec::new();

        for (idx, opt) in options.iter().enumerate() {
            let evaluated = self.eval_expr(opt)?;
            for (prev_idx, prev_exact) in canonical_values.iter().enumerate() {
                if self.expressions_equivalent(&evaluated.exact, prev_exact)? {
                    return Err(HydrogenError::Validation(format!(
                        "duplicate MCQ options at indices {prev_idx} ('{}') and {idx} ('{opt}') both equal {}",
                        options[prev_idx], evaluated.exact
                    )));
                }
            }
            if self.expressions_equivalent(&evaluated.exact, expected_exact)? {
                matching_indices.push(idx);
            }
            canonical_values.push(evaluated.exact);
        }

        match matching_indices.as_slice() {
            [sole_idx] => Ok(*sole_idx),
            [] => Err(HydrogenError::Validation(format!(
                "none of the MCQ options match expected exact answer '{expected_exact}'"
            ))),
            many => Err(HydrogenError::Validation(format!(
                "multiple MCQ options {many:?} match expected exact answer '{expected_exact}'"
            ))),
        }
    }
}

/// Normalize plain-ASCII mixed numbers (`2 3/4` -> `(2 + 3/4)`, `-2 3/4` -> `-(2 + 3/4)`)
/// when `allow_mixed_fractions` is active and the expression is plain ASCII (non-LaTeX).
pub fn normalize_ascii_mixed_fractions(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + 8);
    let mut i = 0;

    while i < len {
        let prev_ok = if i == 0 {
            true
        } else {
            let p = bytes[i - 1];
            !p.is_ascii_alphanumeric() && p != b'.' && p != b'/' && p != b'_'
        };

        if prev_ok && bytes[i].is_ascii_digit() {
            let w_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let w_end = i;

            // Must not be a decimal (`2.5`) or simple fraction (`2/3`)
            let space_start = i;
            while i < len && bytes[i] == b' ' {
                i += 1;
            }
            let has_space = i > space_start;

            if has_space && i < len && bytes[i].is_ascii_digit() {
                let n_start = i;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let n_end = i;

                if i < len && bytes[i] == b'/' {
                    i += 1;
                    if i < len && bytes[i].is_ascii_digit() {
                        let d_start = i;
                        while i < len && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                        let d_end = i;
                        let next_ok = if i == len {
                            true
                        } else {
                            let nx = bytes[i];
                            !nx.is_ascii_alphanumeric() && nx != b'.' && nx != b'/' && nx != b'_'
                        };
                        if next_ok {
                            out.push('(');
                            out.push_str(&input[w_start..w_end]);
                            out.push_str(" + ");
                            out.push_str(&input[n_start..n_end]);
                            out.push('/');
                            out.push_str(&input[d_start..d_end]);
                            out.push(')');
                            continue;
                        }
                    }
                }
                // Rewind back to right after the whole number so normal scanning resumes
                i = w_end;
            } else {
                i = w_end;
            }
            out.push_str(&input[w_start..w_end]);
            continue;
        }

        // Advance one UTF-8 character safely
        if let Some(ch) = input[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_exact_rationals_mixed_fractions_html_id_and_mcq_gate() -> Result<()> {
        let theta = ThetaService::new(StagePresetConfig::gyara_11plus_maths());

        // 1a. Native LaTeX mixed fraction (`2\frac{3}{4} + 1\frac{1}{2} = 17/4`)
        let latex_sum = theta.eval_expr(r"2\frac{3}{4} + 1\frac{1}{2}")?;
        assert_eq!(latex_sum.exact, "17/4");

        // 1b. Plain-ASCII mixed fraction (`2 3/4 + 1 1/2 = 17/4` and `-2 3/4 = -11/4`)
        let ascii_sum = theta.eval_expr("2 3/4 + 1 1/2")?;
        assert_eq!(ascii_sum.exact, "17/4");
        let neg_mix = theta.eval_expr("-2 3/4")?;
        assert_eq!(neg_mix.exact, "-11/4");

        // 1c. `\htmlId` KaTeX token anchors on evaluated expressions
        let alg_expr = theta.eval_expr("3*x + 17/4")?;
        assert!(
            alg_expr.katex.contains("\\htmlId"),
            "anchor_tokens: true must emit \\htmlId KaTeX anchors, got: {}",
            alg_expr.katex
        );

        // 2. Degree-mode trigonometry (`sin(30) == 1/2`)
        let trig = theta.eval_expr("sin(30)")?;
        assert_eq!(trig.exact, "1/2");

        // 3. 11+ Algebraic equation solving (`3x + 7 = 28 -> x = 7` & simultaneous `a+b=52, a-b=14`)
        let solve_val = theta.dispatch_tool(
            "calc.solve",
            &serde_json::json!({
                "equations": ["3*x + 7 = 28"],
                "solve_for": ["x"]
            }),
        )?;
        assert_eq!(solve_val.get("ok").and_then(Value::as_bool), Some(true));

        let sim = theta.solve_equations(&["a + b = 52", "a - b = 14"], &["a", "b"], true)?;
        let sols = sim
            .solutions
            .as_ref()
            .ok_or_else(|| HydrogenError::Validation("missing solutions".into()))?;
        assert_eq!(sols.len(), 1);
        assert_eq!(sols[0].get("a").map(|t| t.exact.as_str()), Some("33"));
        assert_eq!(sols[0].get("b").map(|t| t.exact.as_str()), Some("19"));

        // 4. 5-option (`A-E`) MCQ distractor uniqueness & exact-answer gate
        let correct_idx = theta.verify_mcq_options(
            &["3 1/4", "4", "4 1/4", "4 1/2", "5 1/4"],
            "17/4",
        )?;
        assert_eq!(correct_idx, 2);

        // Duplicate option (`4 1/4` and `17/4`) must be rejected deterministically!
        let dup_err = theta.verify_mcq_options(
            &["3 1/4", "17/4", "4 1/4", "4 1/2", "5"],
            "17/4",
        );
        assert!(dup_err.is_err());

        Ok(())
    }
}
