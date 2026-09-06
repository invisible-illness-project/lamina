import 'dart:math';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaPpg Tests', () {
    test('clean and findPeaks process PPG signals', () async {
      final signal = Float64List.fromList(
        List.generate(300, (i) {
          final mod = i % 100;
          if (mod == 30) return 3.0; // Synthetic PPG peak
          return 0.2 * sin(i * 0.1);
        }),
      );

      final cleaned = await Lamina.ppg.clean(signal, 100.0);
      expect(cleaned.length, equals(300));

      final peaks = await Lamina.ppg.findPeaks(cleaned, 100.0);
      expect(peaks.isNotEmpty, isTrue);

      final mask = await Lamina.ppg.findPeaksMask(cleaned, 100.0);
      expect(mask.length, equals(300));
    });
  });
}
