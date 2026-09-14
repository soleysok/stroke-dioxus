//! Mandarin pronunciation via the browser's own speech synthesiser.
//!
//! `speechSynthesis` is free and needs no key, which is why the React app uses it
//! too. Support is not universal, so every caller must cope with
//! [`is_supported`] returning false and still be usable.

use wasm_bindgen::JsCast;
use web_sys::{SpeechSynthesis, SpeechSynthesisUtterance, SpeechSynthesisVoice};

/// Slower than conversational: the point is to hear each syllable's tone.
const RATE: f32 = 0.8;

fn synth() -> Option<SpeechSynthesis> {
    web_sys::window()?.speech_synthesis().ok()
}

pub fn is_supported() -> bool {
    synth().is_some()
}

/// Best installed Mandarin voice: prefer mainland Chinese, then any Mandarin,
/// then any Chinese at all.
fn pick_voice(synth: &SpeechSynthesis) -> Option<SpeechSynthesisVoice> {
    let voices: Vec<SpeechSynthesisVoice> = synth
        .get_voices()
        .iter()
        .filter_map(|v| v.dyn_into::<SpeechSynthesisVoice>().ok())
        .collect();

    let lang = |v: &SpeechSynthesisVoice| v.lang().to_lowercase().replace('_', "-");
    voices
        .iter()
        .find(|v| lang(v).starts_with("zh-cn") && v.local_service())
        .or_else(|| voices.iter().find(|v| lang(v).starts_with("zh-cn")))
        .or_else(|| voices.iter().find(|v| lang(v).starts_with("cmn")))
        .or_else(|| voices.iter().find(|v| lang(v).starts_with("zh")))
        .cloned()
}

/// Speak one line of Chinese, cancelling anything already playing so that rapid
/// taps do not queue up a backlog.
pub fn speak(text: &str) {
    let Some(synth) = synth() else { return };
    if text.is_empty() {
        return;
    }
    synth.cancel();

    let Ok(utterance) = SpeechSynthesisUtterance::new_with_text(text) else {
        return;
    };
    if let Some(voice) = pick_voice(&synth) {
        utterance.set_lang(&voice.lang());
        utterance.set_voice(Some(&voice));
    } else {
        utterance.set_lang("zh-CN");
    }
    utterance.set_rate(RATE);
    synth.speak(&utterance);
}
