import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaFeatures Tests', () {
    test('extractCardiac extracts features over window', () async {
      final rPeaks = [100, 200, 300, 400]; // 1s spacing @ 100Hz = 60 BPM
      const window = FeatureWindow(startTimeSec: 0.0, endTimeSec: 5.0);
      final features = await Lamina.features.extractCardiac(
        rPeaks,
        100.0,
        0.0,
        window,
      );

      expect(features.beatCount, equals(4));
      expect(features.meanHrBpm, isNotNull);
      expect(features.meanHrBpm!, closeTo(60.0, 1.0));
    });

    test('extractEda extracts features over window', () async {
      final tonic = Float64List.fromList([2.0, 2.1, 2.2, 2.3, 2.4]);
      final phasic = Float64List.fromList([0.1, 0.2, 0.3, 0.2, 0.1]);
      const window = FeatureWindow(startTimeSec: 0.0, endTimeSec: 0.05);

      final features = await Lamina.features.extractEda(
        tonic,
        phasic,
        [],
        100.0,
        0.0,
        window,
      );
      expect(features.meanTonicUs, isNotNull);
    });

    test('extractRespiration extracts features over window', () async {
      final cycles = [
        const RespirationCycle(
          inspirationIndex: 100,
          expirationIndex: 150,
          nextInspirationIndex: 200,
          durationSec: 1.0,
          respiratoryRateBpm: 60.0,
          amplitude: 1.0,
        ),
      ];
      const window = FeatureWindow(startTimeSec: 0.0, endTimeSec: 3.0);
      final features = await Lamina.features.extractRespiration(
        cycles,
        100.0,
        0.0,
        window,
      );
      expect(features.cycleCount, equals(1));
    });
  });
}
