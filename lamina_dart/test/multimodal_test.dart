import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';

import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaMultimodal Tests', () {
    test('pulseTiming calculates PAT/PTT delay', () async {
      final ecgPeaks = [100, 200, 300];
      final ppgPeaks = [120, 220, 320]; // 20 samples = 200ms delay @ 100Hz

      final timings = await Lamina.multimodal.pulseTiming(
        ecgPeaks,
        100.0,
        0.0,
        ppgPeaks,
        100.0,
        0.0,
      );

      expect(timings.length, equals(3));
      expect(timings[0].pulseDelaySec, closeTo(0.20, 1e-4));
    });

    test('phaseCoupling computes circular statistics', () async {
      final phases = Float64List.fromList([0.0, 0.1, -0.1, 0.05]);
      final result = await Lamina.multimodal.phaseCoupling(phases);

      expect(result.vectorLength, greaterThan(0.9));
      expect(result.sampleCount, equals(4));
    });

    test('rsa estimates respiratory sinus arrhythmia', () async {
      final rPeaks = [100, 180, 260, 340, 420];
      final cycles = [
        const RespirationCycle(
          inspirationIndex: 80,
          expirationIndex: 180,
          nextInspirationIndex: 280,
          durationSec: 2.0,
          respiratoryRateBpm: 30.0,
          amplitude: 1.0,
        ),
        const RespirationCycle(
          inspirationIndex: 280,
          expirationIndex: 380,
          nextInspirationIndex: 480,
          durationSec: 2.0,
          respiratoryRateBpm: 30.0,
          amplitude: 1.0,
        ),
      ];

      final rsa = await Lamina.multimodal.rsa(
        rPeaks,
        100.0,
        0.0,
        cycles,
        100.0,
        0.0,
      );
      expect(rsa.validBeats, greaterThanOrEqualTo(3));
    });
  });
}
