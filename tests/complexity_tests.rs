use lamina::complexity::entropy::sample_entropy;
use ndarray::array;

#[test]
fn test_sample_entropy_deterministic() {
    let signal = array![1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0];
    let sampen = sample_entropy(&signal, 2, 0.2).expect("Sample entropy failed");

    assert!(sampen.is_finite());
    assert!(
        sampen < 0.1,
        "Sample Entropy for deterministic sequence should be low"
    );
}

#[test]
fn test_sample_entropy_random() {
    let signal = array![1.0, 0.4, 3.2, 1.1, 4.4, 0.1, -1.0, 2.3, 0.0, 1.1, 3.0];
    let sampen = sample_entropy(&signal, 2, 1.0).expect("Sample entropy failed");

    if sampen.is_finite() {
        assert!(sampen >= 0.0);
    }
}
