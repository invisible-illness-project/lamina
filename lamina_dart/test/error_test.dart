import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('SignalError Propagation Tests', () {
    test('empty signal throws SignalError exception', () async {
      final emptySignal = Float64List(0);
      expect(
        () async => await Lamina.signal.smoothMovingAverage(emptySignal, 3),
        throwsA(isA<SignalError>()),
      );
    });

    test('invalid sampling rate <= 0 throws SignalError exception', () async {
      final signal = Float64List.fromList([1.0, 2.0, 3.0]);
      expect(
        () async => await Lamina.ecg.clean(signal, -10.0),
        throwsA(isA<SignalError>()),
      );
    });

    test('non-finite input throws SignalError exception', () async {
      final signal = Float64List.fromList([1.0, double.nan, 3.0]);
      expect(
        () async => await Lamina.signal.smoothMovingAverage(signal, 3),
        throwsA(isA<SignalError>()),
      );
    });

    test(
      'invalid filter cutoff frequencies throw SignalError exception',
      () async {
        final signal = Float64List.fromList([1.0, 2.0, 3.0, 4.0, 5.0]);
        expect(
          () async => await Lamina.signal.filter(
            signal,
            100.0,
            2,
            lowcut: 50.0,
            highcut: 10.0,
          ),
          throwsA(isA<SignalError>()),
        );
      },
    );
  });
}
