import 'dart:math';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('Numerical Parity Validation Tests', () {
    test('Moving average centered window matches reference math', () async {
      // Input: [1.0, 2.0, 3.0, 4.0, 5.0]
      // Window = 3
      // Index 0: (1+2)/2 = 1.5
      // Index 1: (1+2+3)/3 = 2.0
      // Index 2: (2+3+4)/3 = 3.0
      // Index 3: (3+4+5)/3 = 4.0
      // Index 4: (4+5)/2 = 4.5
      final signal = Float64List.fromList([1.0, 2.0, 3.0, 4.0, 5.0]);
      final res = await Lamina.signal.smoothMovingAverage(signal, 3);
      expect(res[0], closeTo(1.5, 1e-6));
      expect(res[1], closeTo(2.0, 1e-6));
      expect(res[2], closeTo(3.0, 1e-6));
      expect(res[3], closeTo(4.0, 1e-6));
      expect(res[4], closeTo(4.5, 1e-6));
    });

    test(
      'HRV RMSSD and Mean NN match reference mathematical definitions',
      () async {
        // Intervals: [800.0, 850.0, 780.0, 820.0]
        // Diff: [50, -70, 40]
        // Squared diffs: [2500, 4900, 1600] -> sum = 9000
        // Mean sq diff = 9000 / 3 = 3000
        // RMSSD = sqrt(3000) = 54.77225575051661
        final intervals = Float64List.fromList([800.0, 850.0, 780.0, 820.0]);
        final rmssd = await Lamina.hrv.rmssd(intervals);
        final mean = await Lamina.hrv.meanNn(intervals);

        expect(rmssd, closeTo(sqrt(3000.0), 1e-5));
        expect(mean, closeTo(812.5, 1e-5));
      },
    );

    test(
      'Phase coupling circular resultant vector matches analytical formula',
      () async {
        // Phases = [0, pi/2] -> cos = [1, 0], sin = [0, 1]
        // Mean cos = 0.5, Mean sin = 0.5 -> R = sqrt(0.5^2 + 0.5^2) = sqrt(0.5) = 0.70710678
        final phases = Float64List.fromList([0.0, pi / 2]);
        final result = await Lamina.multimodal.phaseCoupling(phases);

        expect(result.vectorLength, closeTo(sqrt(0.5), 1e-5));
        expect(result.meanPhaseRad, closeTo(pi / 4, 1e-5));
      },
    );
  });
}
