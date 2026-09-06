import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaHrv Tests', () {
    test('peaksToIntervals converts sample indices to ms intervals', () async {
      final peaks = [
        100,
        200,
        300,
        400,
      ]; // 100 sample intervals @ 100Hz = 1000ms each
      final intervals = await Lamina.hrv.peaksToIntervals(peaks, 100.0, 500);
      expect(intervals.length, equals(3));
      expect(intervals[0], closeTo(1000.0, 1e-5));
      expect(intervals[1], closeTo(1000.0, 1e-5));
    });

    test('rmssd, meanNn, sdnn, and pnn50 compute valid HRV metrics', () async {
      final intervals = Float64List.fromList([
        800.0,
        850.0,
        780.0,
        820.0,
        790.0,
      ]);

      final mean = await Lamina.hrv.meanNn(intervals);
      expect(mean, closeTo(808.0, 1e-5));

      final rmssd = await Lamina.hrv.rmssd(intervals);
      expect(rmssd, greaterThan(0.0));

      final sdnn = await Lamina.hrv.sdnn(intervals);
      expect(sdnn, greaterThan(0.0));

      final pnn50 = await Lamina.hrv.pnn50(intervals);
      expect(pnn50, greaterThanOrEqualTo(0.0));
    });
  });
}
