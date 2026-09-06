import 'dart:math';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaEda Tests', () {
    test('clean, phasic, and decompose process EDA signals', () async {
      final signal = Float64List.fromList(
        List.generate(200, (i) => 2.0 + 0.5 * sin(i * 0.05)),
      );
      final cleaned = await Lamina.eda.clean(signal, 100.0);
      expect(cleaned.length, equals(200));

      final phasic = await Lamina.eda.phasic(cleaned, 100.0);
      expect(phasic.length, equals(200));

      final decomposed = await Lamina.eda.decompose(cleaned, 100.0);
      expect(decomposed.tonic.length, equals(200));
      expect(decomposed.phasic.length, equals(200));
    });

    test('findPeaks and findPeaksEvents detect SCR events', () async {
      final phasicSignal = Float64List.fromList(
        List.generate(400, (i) {
          final pulse1 = 1.5 * exp(-pow((i - 100) / 15.0, 2));
          final pulse2 = 1.2 * exp(-pow((i - 250) / 15.0, 2));
          return 0.01 + pulse1 + pulse2;
        }),
      );

      final peaks = await Lamina.eda.findPeaks(phasicSignal, 100.0);
      expect(peaks.length, greaterThanOrEqualTo(1));

      final events = await Lamina.eda.findPeaksEvents(phasicSignal, 100.0);
      expect(events.length, greaterThanOrEqualTo(1));
    });
  });
}
