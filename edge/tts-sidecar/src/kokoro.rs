//! Kokoro-82M ONNX inference.
//!
//! ONNX I/O contract (`model_quantized.onnx`):
//!   Inputs:
//!     `"input_ids"` : int64[1, seq_len]  — token IDs with 0 BOS/EOS padding
//!     `"style"`     : f32[1, 256]        — voice style vector
//!     `"speed"`     : f32[1]             — speed multiplier (1.0 = normal)
//!   Output:
//!     `"waveform"`  : f32[1, num_samples] — raw PCM at 24 kHz

use std::collections::HashMap;
use std::path::PathBuf;

use misaki_rs::G2P;
use ndarray::{Array1, Array2};
use ort::{session::Session, value::Tensor};

use crate::vocab::phonemes_to_ids;

/// Maximum phoneme sequence length accepted by the model.
pub const MAX_PHONEME_LEN: usize = 510;

/// Load an ONNX session from `model_path`.
pub fn load_session(model_path: &str) -> Result<Session, String> {
    Session::builder()
        .map_err(|e| format!("ort session builder: {e}"))?
        .commit_from_file(PathBuf::from(model_path))
        .map_err(|e| format!("load ONNX model '{model_path}': {e}"))
}

/// Convert `text` → phoneme token IDs using misaki-rs (English US, no espeak).
///
/// `g2p` is passed by reference so the caller can construct it once and reuse
/// it across requests (G2P construction loads data and is expensive).
pub fn text_to_token_ids(
    text: &str,
    vocab: &HashMap<char, i64>,
    g2p: &G2P,
) -> Result<Vec<i64>, String> {
    let (phonemes, _tokens) = g2p.g2p(text).map_err(|e| format!("G2P error: {e:?}"))?;

    let ids = phonemes_to_ids(&phonemes, vocab)?;

    if ids.len() > MAX_PHONEME_LEN {
        return Err(format!(
            "phoneme sequence too long: {} > {MAX_PHONEME_LEN}",
            ids.len()
        ));
    }
    Ok(ids)
}

/// Run inference and return raw f32 PCM samples.
///
/// `token_ids` must NOT already include BOS/EOS padding — this function
/// prepends and appends a `0` sentinel, matching the prototype behaviour.
pub fn synthesize(
    session: &mut Session,
    token_ids: &[i64],
    style: &Array2<f32>,
    speed: f32,
) -> Result<Vec<f32>, String> {
    // Wrap token IDs with BOS/EOS sentinel 0.
    let padded: Vec<i64> = std::iter::once(0i64)
        .chain(token_ids.iter().copied())
        .chain(std::iter::once(0i64))
        .collect();
    let seq_len = padded.len();

    let input_ids_arr = Array2::from_shape_vec((1, seq_len), padded)
        .map_err(|e| format!("input_ids shape: {e}"))?;
    let speed_arr = Array1::from_vec(vec![speed]);

    let input_ids_tensor =
        Tensor::from_array(input_ids_arr).map_err(|e| format!("input_ids tensor: {e}"))?;
    let style_tensor =
        Tensor::from_array(style.clone()).map_err(|e| format!("style tensor: {e}"))?;
    let speed_tensor =
        Tensor::from_array(speed_arr).map_err(|e| format!("speed tensor: {e}"))?;

    let outputs = session
        .run(ort::inputs![
            "input_ids" => input_ids_tensor,
            "style"     => style_tensor,
            "speed"     => speed_tensor,
        ])
        .map_err(|e| format!("ort run: {e}"))?;

    let (_shape, waveform) = outputs["waveform"]
        .try_extract_tensor::<f32>()
        .map_err(|e| format!("extract waveform: {e}"))?;

    Ok(waveform.to_vec())
}
