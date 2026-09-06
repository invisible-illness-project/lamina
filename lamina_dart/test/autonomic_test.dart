import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  setUpAll(() async {
    await Lamina.init();
  });

  group('LaminaAutonomic Tests', () {
    test(
      'createEstimator instantiates stateful handle and estimates state',
      () async {
        final estimator = await Lamina.autonomic.createEstimator();
        expect(estimator, isNotNull);

        final state = await estimator.estimateFromVector();
        expect(state, isNotNull);
        expect(state.cardiac.beatCount, equals(0));
      },
    );
  });
}
