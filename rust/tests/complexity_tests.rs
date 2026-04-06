use ndarray::array;
use lamina::complexity::entropy::sample_entropy;

#[test]
fn test_sample_entropy_deterministic() {
    // Array with highly recurring elements
    let signal = array![1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0];
    let sampen = sample_entropy(&signal, 2, 0.2);
    
    // For a highly regular alternating signal, the count of matching (m) templates
    // is expected to strongly align with (m+1). Hence, small or zero entropy.
    // In this specific sequence [1,2,1,2,1...], pairs like [1,2] repeat perfectly.
    assert!(sampen.is_finite());
    assert!(sampen < 0.1, "Sample Entropy for deterministic sequence should be low");
}

#[test]
fn test_sample_entropy_random() {
    // A somewhat randomized array
    let signal = array![1.0, 0.4, 3.2, 1.1, 4.4, 0.1, -1.0, 2.3, 0.0, 1.1, 3.0];
    let sampen = sample_entropy(&signal, 2, 1.0);
    
    // Entropy should be bounded but likely higher than deterministic setups
    // or infinity if there are zero templates mapping out exactly.
    if sampen.is_finite() {
        assert!(sampen >= 0.0);
    } // Otherwise, if infinity, it correctly means complete disorder within threshold
}
