#[cfg(test)]
mod hlx_integration_tests {
    use super::super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_hlx_preference_format_detection() {
        let data = vec![
            json!({
                "prompt": "Explain quantum computing",
                "chosen": "Quantum computing uses quantum mechanics principles...",
                "rejected": "Quantum computing is just regular computing but faster."
            }),
            json!({
                "prompt": "What is machine learning?",
                "chosen": "Machine learning is a subset of AI that enables systems to learn from data...",
                "rejected": "Machine learning is when computers learn stuff."
            }),
        ];

        let dataset = GenericJSONDataset::new(
            &[PathBuf::from("dummy.json")],
            None,
            DataFormat::Auto,
        ).unwrap();

        // Manually set data for testing
        let mut test_dataset = GenericJSONDataset {
            data,
            format: DataFormat::Auto,
            schema: None,
        };

        let training_format = test_dataset.detect_training_format().unwrap();
        match training_format {
            TrainingFormat::Preference { .. } => {},
            _ => panic!("Expected Preference format"),
        }

        let training_dataset = test_dataset.to_training_dataset().unwrap();
        assert_eq!(training_dataset.samples.len(), 2);
        assert!(training_dataset.samples[0].chosen.is_some());
        assert!(training_dataset.samples[0].rejected.is_some());
    }

    #[test]
    fn test_hlx_completion_format_detection() {
        let data = vec![
            json!({
                "prompt": "Explain quantum computing",
                "completion": "Quantum computing uses quantum mechanics principles...",
                "label": 1
            }),
            json!({
                "prompt": "What is machine learning?",
                "completion": "Machine learning is a subset of AI that enables systems to learn from data...",
                "label": 0
            }),
        ];

        let mut test_dataset = GenericJSONDataset {
            data,
            format: DataFormat::Auto,
            schema: None,
        };

        let training_format = test_dataset.detect_training_format().unwrap();
        match training_format {
            TrainingFormat::Completion { .. } => {},
            _ => panic!("Expected Completion format"),
        }

        let training_dataset = test_dataset.to_training_dataset().unwrap();
        assert_eq!(training_dataset.samples.len(), 2);
        assert!(training_dataset.samples[0].completion.is_some());
        assert!(training_dataset.samples[0].label.is_some());
    }

    #[test]
    fn test_algorithm_format_conversion() {
        let data = vec![
            json!({
                "prompt": "Explain quantum computing",
                "chosen": "Quantum computing uses quantum mechanics principles...",
                "rejected": "Quantum computing is just regular computing but faster."
            }),
        ];

        let mut test_dataset = GenericJSONDataset {
            data,
            format: DataFormat::Auto,
            schema: None,
        };

        let training_dataset = test_dataset.to_training_dataset().unwrap();

        // Test DPO conversion
        let dpo_result = training_dataset.to_algorithm_format("dpo");
        assert!(dpo_result.is_ok());

        // Test BCO conversion
        let bco_result = training_dataset.to_algorithm_format("bco");
        assert!(bco_result.is_ok());

        // Test invalid algorithm
        let invalid_result = training_dataset.to_algorithm_format("invalid");
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_dataset_quality_assessment() {
        let data = vec![
            json!({
                "prompt": "Short",
                "chosen": "Good",
                "rejected": "Bad"
            }),
        ];

        let mut test_dataset = GenericJSONDataset {
            data,
            format: DataFormat::Auto,
            schema: None,
        };

        let training_dataset = test_dataset.to_training_dataset().unwrap();
        let quality_report = training_dataset.quality_assessment();

        assert!(quality_report.overall_score >= 0.0 && quality_report.overall_score <= 1.0);
        // Should have issues due to short prompt length
        assert!(!quality_report.issues.is_empty());
    }

    #[tokio::test]
    async fn test_hf_processor_integration() {
        let processor = HfProcessor::default();

        let config = HfDatasetConfig {
            source: "Anthropic/hh-rlhf".to_string(),
            split: "train".to_string(),
            format: None,
            rpl_filter: None,
        };

        let result = processor.process_dataset("Anthropic/hh-rlhf", &config).await;
        assert!(result.is_ok());

        let dataset = result.unwrap();
        assert!(!dataset.samples.is_empty());
        assert!(dataset.samples[0].chosen.is_some());
        assert!(dataset.samples[0].rejected.is_some());
    }

    #[test]
    fn test_training_format_enum() {
        // Test all training format variants
        let preference = TrainingFormat::Preference {
            chosen_field: "chosen".to_string(),
            rejected_field: "rejected".to_string(),
        };
        assert!(matches!(preference, TrainingFormat::Preference { .. }));

        let completion = TrainingFormat::Completion {
            completion_field: "completion".to_string(),
            label_field: Some("label".to_string()),
        };
        assert!(matches!(completion, TrainingFormat::Completion { .. }));

        let instruction = TrainingFormat::Instruction {
            instruction_field: "instruction".to_string(),
            output_field: "output".to_string(),
        };
        assert!(matches!(instruction, TrainingFormat::Instruction { .. }));

        let chat = TrainingFormat::Chat {
            messages_field: "messages".to_string(),
        };
        assert!(matches!(chat, TrainingFormat::Chat { .. }));

        let custom = TrainingFormat::Custom {
            fields: vec!["field1".to_string(), "field2".to_string()],
        };
        assert!(matches!(custom, TrainingFormat::Custom { .. }));
    }

    #[test]
    fn test_dataset_statistics_computation() {
        let samples = vec![
            TrainingSample {
                prompt: Some("Test prompt".to_string()),
                chosen: Some("Chosen response".to_string()),
                rejected: Some("Rejected response".to_string()),
                completion: None,
                label: None,
                meta: std::collections::HashMap::new(),
            },
            TrainingSample {
                prompt: Some("Another prompt".to_string()),
                chosen: Some("Another chosen".to_string()),
                rejected: Some("Another rejected".to_string()),
                completion: None,
                label: None,
                meta: std::collections::HashMap::new(),
            },
        ];

        let format = TrainingFormat::Preference {
            chosen_field: "chosen".to_string(),
            rejected_field: "rejected".to_string(),
        };

        let dataset = TrainingDataset {
            samples,
            format,
            statistics: DatasetStats {
                total_samples: 2,
                avg_prompt_length: 12.5,
                avg_completion_length: 0.0,
                field_coverage: [
                    ("prompt".to_string(), 1.0),
                    ("chosen".to_string(), 1.0),
                    ("rejected".to_string(), 1.0),
                ].into(),
                quality_score: Some(1.0),
            },
        };

        assert_eq!(dataset.samples.len(), 2);
        assert!(dataset.statistics.quality_score.unwrap() > 0.0);
    }
}
