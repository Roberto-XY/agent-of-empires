/// Structured progress events emitted during session creation.
#[derive(Debug, Clone)]
pub enum CreationProgress {
    StepStarted {
        source: CreationProgressSource,
        label: String,
    },
    Output {
        source: CreationProgressSource,
        line: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationProgressSource {
    Hook,
    Compose,
    System,
}
