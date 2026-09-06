import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaRppg Tests', () {
    test('extractFromOpticalSignal processes camera RGB channels', () async {
      final timestamps = Float64List.fromList([0.0, 0.033, 0.066, 0.100]);
      final red = Float64List.fromList([150.0, 152.0, 151.0, 153.0]);
      final green = Float64List.fromList([100.0, 103.0, 101.0, 104.0]);
      final blue = Float64List.fromList([90.0, 91.0, 90.0, 92.0]);
      final validPixels = [1000, 1000, 1000, 1000];

      final result = await Lamina.rppg.extractFromOpticalSignal(
        timestamps,
        red,
        green,
        blue,
        validPixels,
      );

      expect(result.timestampsSec.length, equals(4));
      expect(result.pulseSignal.length, equals(4));
      expect(result.meanQuality, greaterThan(0.0));
    });
  });
}
