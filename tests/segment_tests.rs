use lamina::signal::segment::{IncompleteTailPolicy, signal_segment, signal_segment_duration};
use ndarray::Array1;

#[test]
fn test_signal_segment_tail_policies() {
    // 25 samples total, window = 10, step = 10 -> windows at 0..10, 10..20, tail at 20..25 (5 samples)
    let sig: Vec<f64> = (0..25).map(|i| i as f64).collect();
    let arr = Array1::from_vec(sig);

    // 1. DropIncomplete -> 2 windows of length 10
    let drop_segs = signal_segment(&arr, 10, 10, IncompleteTailPolicy::DropIncomplete).unwrap();
    assert_eq!(drop_segs.len(), 2);
    assert_eq!(drop_segs[0].len(), 10);
    assert_eq!(drop_segs[1].len(), 10);
    assert_eq!(drop_segs[0][0], 0.0);
    assert_eq!(drop_segs[1][0], 10.0);

    // 2. PadZeros -> 3 windows of length 10
    let pad_segs = signal_segment(&arr, 10, 10, IncompleteTailPolicy::PadZeros).unwrap();
    assert_eq!(pad_segs.len(), 3);
    assert_eq!(pad_segs[2].len(), 10);
    assert_eq!(pad_segs[2][0], 20.0);
    assert_eq!(pad_segs[2][4], 24.0);
    assert_eq!(pad_segs[2][5], 0.0); // Zero padded

    // 3. KeepPartial -> 3 windows (10, 10, 5)
    let keep_segs = signal_segment(&arr, 10, 10, IncompleteTailPolicy::KeepPartial).unwrap();
    assert_eq!(keep_segs.len(), 3);
    assert_eq!(keep_segs[2].len(), 5);
    assert_eq!(keep_segs[2][4], 24.0);
}

#[test]
fn test_signal_segment_duration() {
    // 125 Hz sampling rate, 35 seconds of data = 4375 samples
    let fs = 125.0;
    let n = (35.0 * fs) as usize;
    let arr = Array1::zeros(n);

    // 10s windows, 10s stride, DropIncomplete -> exactly 3 windows of 1250 samples
    let segs = signal_segment_duration(&arr, fs, 10.0, 10.0, IncompleteTailPolicy::DropIncomplete).unwrap();
    assert_eq!(segs.len(), 3);
    for seg in segs {
        assert_eq!(seg.len(), 1250);
    }
}
