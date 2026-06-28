// See plans/09-core-types.md §2

/// Modality tags for raw perception input/output.
///
/// Used by the `Ground` opcode to specify the kind of raw data being
/// attached to a ConceptVector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Modality {
    /// Visual input (images, video).
    Vision,
    /// Audio input (sound, speech).
    Audio,
    /// Text input (natural language, code).
    Text,
    /// Proprioceptive input (robot joint angles, sensor readings).
    Proprioception,
    /// Custom modality (user-defined u8 tag).
    Custom(u8),
}
