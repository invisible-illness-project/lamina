import 'dart:math';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaRsp Tests', () {
    test(
      'clean, findPeaks, cycles, and rate process respiratory signals',
      () async {
        final signal = Float64List.fromList(
          List.generate(2000, (i) => sin(2 * pi * 0.25 * i / 100.0)),
        );
        final cleaned = await Lamina.rsp.clean(signal, 100.0);
        expect(cleaned.length, equals(2000));

        final peaks = await Lamina.rsp.findPeaks(cleaned, 100.0);
        expect(peaks.length, greaterThanOrEqualTo(2));

        final cycles = await Lamina.rsp.cycles(cleaned, 100.0);
        expect(cycles.length, greaterThanOrEqualTo(1));

        final rate = await Lamina.rsp.rate(cleaned, 100.0);
        expect(rate.length, equals(2000));
      },
    );
  });
}
