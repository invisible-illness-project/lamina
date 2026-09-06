import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaComplexity Tests', () {
    test('sampleEntropy calculates SampEn on deterministic signal', () async {
      final signal = Float64List.fromList([
        1.0,
        2.0,
        1.0,
        2.0,
        1.0,
        2.0,
        1.0,
        2.0,
        1.0,
        2.0,
      ]);
      final sampEn = await Lamina.complexity.sampleEntropy(signal, 2, 0.2);
      expect(sampEn, isNotNull);
      expect(sampEn, isNot(double.nan));
    });
  });
}
