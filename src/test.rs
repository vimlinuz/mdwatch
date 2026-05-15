#[cfg(test)]
mod tests {
    use crate::utils::rewrite_image_paths;

    struct TestCase {
        input: &'static str,
        expected: &'static str,
    }

    #[test]
    fn test_rewrite_image_paths() {
        let test_cases = [
            TestCase {
                input: r#"<img src="image.png" alt="Image">"#,
                expected: r#"<img src="/_local_image/image.png" alt="Image">"#,
            },
            TestCase {
                input: r#"<img src="http://example.com/image.png" alt="Remote Image">"#,
                expected: r#"<img src="http://example.com/image.png" alt="Remote Image">"#,
            },
            TestCase {
                input: r#"<img src="data:image/png;base64,..." alt="Data URI">"#,
                expected: r#"<img src="data:image/png;base64,..." alt="Data URI">"#,
            },
            TestCase {
                input: r#"<img src="//example.com/image.png" alt="Protocol-relative URL">"#,
                expected: r#"<img src="//example.com/image.png" alt="Protocol-relative URL">"#,
            },
        ];

        for case in test_cases {
            let result = rewrite_image_paths(case.input);
            assert_eq!(
                result, case.expected,
                "Failed to rewrite image paths for input: {}",
                case.input
            );
        }
    }
}
