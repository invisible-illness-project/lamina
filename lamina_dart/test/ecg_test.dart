import 'dart:math';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaEcg Tests', () {
    test('clean processes ECG signal', () async {
      final signal = Float64List.fromList(
        List.generate(200, (i) => sin(i * 0.1)),
      );
      final cleaned = await Lamina.ecg.clean(signal, 100.0);
      expect(cleaned.length, equals(200));
    });

    test('findPeaks and findPeaksMask detect Pan-Tompkins R-peaks', () async {
      final signal = Float64List.fromList(
        List.generate(300, (i) {
          final mod = i % 100;
          if (mod == 50) return 5.0; // Synthetic R-peak
          return 0.1 * sin(i * 0.1);
        }),
      );

      final peaks = await Lamina.ecg.findPeaks(signal, 100.0);
      expect(peaks.isNotEmpty, isTrue);

      final mask = await Lamina.ecg.findPeaksMask(signal, 100.0);
      expect(mask.length, equals(300));
    });
  });
}
