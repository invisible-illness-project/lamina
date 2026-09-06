import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:lamina_dart/lamina_dart.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await Lamina.init());

  testWidgets('Can call Lamina Rust backend', (WidgetTester tester) async {
    final signal = Float64List.fromList([1.0, 2.0, 3.0, 4.0, 5.0]);
    final smoothed = await Lamina.signal.smoothMovingAverage(signal, 3);
    expect(smoothed.length, equals(5));
    expect(smoothed[0], closeTo(1.5, 1e-5));
  });
}
