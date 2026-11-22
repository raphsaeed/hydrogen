//! Core curriculum domain primitives: `UniversalId` and `QuestionBlueprint` / `SolutionBlueprint`.

pub mod blueprint;
pub mod id;

pub use blueprint::{
    AnswerFormat, AnswerKeyBlueprint, BLUEPRINT_SCHEMA_VERSION, ConceptLinkSpec,
    FinalAnswerBlueprint, ItemAssetSpec, PaperHeaderSpec, PaperPackage, PaperSectionSpec,
    QuestionBlueprint, QuestionPackage, QuestionPartBlueprint, QuestionTopicSpec,
    SolutionBlueprint, SolutionPartBlueprint, SolutionStepBlueprint, StimulusSpec,
    SublevelBlueprint, SubstepBlueprint, ThetaVerificationSpec,
};
pub use id::{ContainerSegment, ItemSegment, MAX_UNIVERSAL_ID_LEN, SubAddress, UniversalId};
