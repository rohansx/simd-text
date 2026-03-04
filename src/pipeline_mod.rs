//! Composable pipeline API — fuse multiple operations into one pass.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::numbers::NumberSpan;

/// Stages that can be enabled in a pipeline.
#[derive(Clone, Copy)]
enum Stage {
    ValidateUtf8,
    SplitLines,
    Classify,
    ExtractNumbers,
}

/// Create a new pipeline builder.
///
/// ```
/// use simd_text::pipeline;
///
/// let pipe = pipeline()
///     .validate_utf8()
///     .split_lines()
///     .build();
///
/// let results = pipe.process(b"hello\nworld\n");
/// assert!(results.utf8_valid);
/// assert_eq!(results.lines.len(), 2);
/// ```
pub fn pipeline() -> PipelineBuilder {
    PipelineBuilder {
        stages: Vec::new(),
        classify_chars: Vec::new(),
    }
}

/// Builder for constructing a fused pipeline.
pub struct PipelineBuilder {
    stages: Vec<Stage>,
    classify_chars: Vec<u8>,
}

impl PipelineBuilder {
    /// Add UTF-8 validation to the pipeline.
    pub fn validate_utf8(mut self) -> Self {
        self.stages.push(Stage::ValidateUtf8);
        self
    }

    /// Add line splitting to the pipeline.
    pub fn split_lines(mut self) -> Self {
        self.stages.push(Stage::SplitLines);
        self
    }

    /// Add character classification to the pipeline.
    pub fn classify(mut self, chars: &[u8]) -> Self {
        self.stages.push(Stage::Classify);
        self.classify_chars = chars.to_vec();
        self
    }

    /// Add number extraction to the pipeline.
    pub fn extract_numbers(mut self) -> Self {
        self.stages.push(Stage::ExtractNumbers);
        self
    }

    /// Build the pipeline.
    pub fn build(self) -> Pipeline {
        let has_classify = self.stages.iter().any(|s| matches!(s, Stage::Classify));
        Pipeline {
            stages: self.stages,
            classifier: if has_classify {
                Some(crate::classify::CharClassifier::new(&self.classify_chars))
            } else {
                None
            },
        }
    }
}

/// A compiled pipeline that processes data through multiple stages.
pub struct Pipeline {
    stages: Vec<Stage>,
    classifier: Option<crate::classify::CharClassifier>,
}

/// Results from a pipeline execution.
pub struct PipelineResults<'a> {
    /// Line ranges `(start, end)` if line splitting was enabled.
    pub lines: Vec<(usize, usize)>,
    /// Matched character positions if classification was enabled.
    pub classifications: Vec<usize>,
    /// Extracted numbers if number extraction was enabled.
    pub numbers: Vec<NumberSpan>,
    /// Whether the data is valid UTF-8 (true if validation wasn't enabled).
    pub utf8_valid: bool,
    /// Reference to the original data.
    _marker: core::marker::PhantomData<&'a [u8]>,
}

impl Pipeline {
    /// Process data through all pipeline stages.
    ///
    /// Currently executes stages sequentially (future: fused single-pass).
    pub fn process<'a>(&self, data: &'a [u8]) -> PipelineResults<'a> {
        let mut results = PipelineResults {
            lines: Vec::new(),
            classifications: Vec::new(),
            numbers: Vec::new(),
            utf8_valid: true,
            _marker: core::marker::PhantomData,
        };

        for stage in &self.stages {
            match stage {
                Stage::ValidateUtf8 => {
                    results.utf8_valid = crate::utf8::validate_utf8(data).is_ok();
                }
                Stage::SplitLines => {
                    results.lines = crate::lines::line_ranges(data).collect();
                }
                Stage::Classify => {
                    if let Some(ref classifier) = self.classifier {
                        results.classifications = classifier.find_all(data);
                    }
                }
                Stage::ExtractNumbers => {
                    results.numbers = crate::numbers::extract_numbers(data);
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_validate_and_split() {
        let pipe = pipeline()
            .validate_utf8()
            .split_lines()
            .build();

        let data = b"hello\nworld\n";
        let results = pipe.process(data);
        assert!(results.utf8_valid);
        assert_eq!(results.lines.len(), 2);
        assert_eq!(results.lines[0], (0, 5));
        assert_eq!(results.lines[1], (6, 11));
    }

    #[test]
    fn pipeline_all_stages() {
        let pipe = pipeline()
            .validate_utf8()
            .split_lines()
            .classify(b",")
            .extract_numbers()
            .build();

        let data = b"name,age\nAlice,30\n";
        let results = pipe.process(data);
        assert!(results.utf8_valid);
        assert_eq!(results.lines.len(), 2);
        assert!(!results.classifications.is_empty()); // commas found
        assert!(!results.numbers.is_empty()); // "30" found
    }

    #[test]
    fn pipeline_invalid_utf8() {
        let pipe = pipeline().validate_utf8().build();
        let data = &[0xFF, 0xFE];
        let results = pipe.process(data);
        assert!(!results.utf8_valid);
    }

    #[test]
    fn pipeline_empty() {
        let pipe = pipeline()
            .validate_utf8()
            .split_lines()
            .build();
        let results = pipe.process(b"");
        assert!(results.utf8_valid);
        assert!(results.lines.is_empty());
    }

    #[test]
    fn pipeline_numbers_only() {
        let pipe = pipeline().extract_numbers().build();
        let data = b"x=42 y=3.14";
        let results = pipe.process(data);
        assert_eq!(results.numbers.len(), 2);
    }

    #[test]
    fn pipeline_classify_empty_chars() {
        let pipe = pipeline().classify(b"").build();
        let data = b"hello world";
        let results = pipe.process(data);
        assert!(results.classifications.is_empty());
    }

    #[test]
    fn pipeline_no_stages() {
        let pipe = pipeline().build();
        let data = b"hello";
        let results = pipe.process(data);
        assert!(results.utf8_valid); // default true when not validated
        assert!(results.lines.is_empty());
        assert!(results.classifications.is_empty());
        assert!(results.numbers.is_empty());
    }
}
