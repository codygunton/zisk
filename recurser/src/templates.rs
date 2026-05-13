use tera::{Context, Tera};

use crate::error::Result;

const AGGREGATOR_TMPL: &str = include_str!("../templates/aggregator.circom.tera");
/// Default `PreparePublics` body — identity passthrough. Used when the caller
/// passes `prepare_publics: None`.
const DEFAULT_PREPARE_PUBLICS: &str = include_str!("../templates/prepare_publics.circom");
/// Default `CheckPublics` body — no-op (no stitching constraints). Used when
/// the caller passes `check_publics: None`.
const DEFAULT_CHECK_PUBLICS: &str = include_str!("../templates/check_publics.circom");

fn render(template_src: &str, ctx: &Context) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("t", template_src)?;
    Ok(tera.render("t", ctx)?)
}

#[derive(Debug, Clone)]
pub struct StarkInputBlocks<'a> {
    pub define_a: &'a str,
    pub define_b: &'a str,
    pub assign_a: &'a str,
    pub assign_b: &'a str,
}

/// Circom bodies for the three publics-handling sub-templates.
///
/// Each field is a fully-rendered Circom template body, injected verbatim into
/// the aggregator. All three templates receive `private_inputs`, so any of
/// them can derive constraints / output values from them.
///
/// - `prepare_publics` is **optional**: when `None`, the built-in
///   identity-passthrough body is used.
/// - `check_publics` is **optional**: when `None`, the built-in no-op body
///   (no stitching constraints) is used.
/// - `aggregate_publics` is **required**: there's no sensible default for how
///   two payloads should combine into one.
///
/// Required signatures:
///
/// - `prepare_publics`: `template PreparePublics(nPublics, nPrivateInputs)` with
///   `signal input publics[nPublics]`, `signal input private_inputs[nPrivateInputs]`,
///   `signal output recurser_publics[nPublics]`.
/// - `check_publics`: `template CheckPublics(nPublics, nPrivateInputs)` with
///   `signal input a_publics[nPublics]`, `signal input b_publics[nPublics]`,
///   `signal input private_inputs[nPrivateInputs]`.
/// - `aggregate_publics`: `template AggregatePublics(nPublics, nPrivateInputs)` with
///   `signal output aggregated_publics[nPublics]`, `signal input a_publics[nPublics]`,
///   `signal input b_publics[nPublics]`, `signal input private_inputs[nPrivateInputs]`.
#[derive(Debug, Clone)]
pub struct CircomTemplates {
    /// Optional — `None` uses the built-in identity passthrough.
    pub prepare_publics: Option<String>,
    /// Optional — `None` uses the built-in no-op body (no stitching).
    pub check_publics: Option<String>,
    pub aggregate_publics: String,
}

/// Generate the top-level aggregator Circom from the recurser/templates assets.
///
/// `n_private_inputs` is the count of side inputs the user's `PreparePublics`
/// expects (free parameter; pass 0 if unused).
///
/// `vadcop_final_zisk_vk_row0` is the inner (leaf) verifier's verkey — the
/// 4-element rootC used to verify leaf proofs. It's baked into the aggregator
/// template as the `rootCVadcopFinalZisk` constant.
///
/// All three [`CircomTemplates`] bodies are required — the aggregator no
/// longer ships any default sub-template generation. See [`CircomTemplates`]
/// for the required signatures.
pub fn gen_aggregator(
    n_private_inputs: usize,
    verifier_filename: &str,
    vadcop_final_zisk_vk_row0: &[String],
    program_vks: &[[String; 4]],
    stark_inputs: &StarkInputBlocks<'_>,
    templates: &CircomTemplates,
) -> Result<String> {
    let n_programs = program_vks.len();

    let mut ctx = Context::new();
    ctx.insert("verifier_filename", verifier_filename);
    ctx.insert("n_private_inputs", &n_private_inputs);
    ctx.insert("n_programs", &n_programs);
    ctx.insert("program_vks", program_vks);
    ctx.insert("root_c_vadcop_final_zisk", &vadcop_final_zisk_vk_row0);
    ctx.insert("aggregate_publics_template", &templates.aggregate_publics);
    ctx.insert(
        "prepare_publics_template",
        templates.prepare_publics.as_deref().unwrap_or(DEFAULT_PREPARE_PUBLICS),
    );
    ctx.insert(
        "check_publics_template",
        templates.check_publics.as_deref().unwrap_or(DEFAULT_CHECK_PUBLICS),
    );
    ctx.insert("define_stark_inputs_a", stark_inputs.define_a);
    ctx.insert("define_stark_inputs_b", stark_inputs.define_b);
    ctx.insert("assign_stark_inputs_a", stark_inputs.assign_a);
    ctx.insert("assign_stark_inputs_b", stark_inputs.assign_b);

    render(AGGREGATOR_TMPL, &ctx)
}
