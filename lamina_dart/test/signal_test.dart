import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaSignal Tests', () {
    test('smoothMovingAverage computes moving average correctly', () async {
      final signal = Float64List.fromList([1.0, 2.0, 3.0, 4.0, 5.0]);
      final smoothed = await Lamina.signal.smoothMovingAverage(signal, 3);
      expect(smoothed.length, equals(5));
      expect(smoothed[0], closeTo(1.5, 1e-5));
      expect(smoothed[2], closeTo(3.0, 1e-5));
      expect(smoothed[4], closeTo(4.5, 1e-5));
    });

    test('filter applies bandpass filter', () async {
      final signal = Float64List.fromList(
        List.generate(100, (i) => i % 2 == 0 ? 1.0 : -1.0),
      );
      final filtered = await Lamina.signal.filter(
        signal,
        100.0,
        2,
        lowcut: 1.0,
        highcut: 10.0,
      );
      expect(filtered.length, equals(100));
    });

    test('findPeaks detects signal local maxima', () async {
      final signal = Float64List.fromList([
        0.0,
        1.0,
        5.0,
        1.0,
        0.0,
        2.0,
        8.0,
        2.0,
        0.0,
      ]);
      final peaks = await Lamina.signal.findPeaks(
        signal,
        config: const PeakDetectionConfig(minHeight: 3.0),
      );
      expect(peaks.length, equals(2));
      expect(peaks, containsAll([2, 6]));
    });
  });
}
