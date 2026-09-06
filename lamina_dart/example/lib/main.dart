import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:lamina_dart/lamina_dart.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatefulWidget {
  const MyApp({super.key});

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {
  String _status = 'Initializing Lamina...';
  double? _rmssd;
  int _peakCount = 0;

  @override
  void initState() {
    super.initState();
    _runDemo();
  }

  Future<void> _runDemo() async {
    try {
      await Lamina.init();
      // Generate synthetic ECG R-peak intervals
      final intervals = Float64List.fromList([
        800.0,
        810.0,
        790.0,
        805.0,
        815.0,
        795.0,
      ]);
      final rmssd = await Lamina.hrv.rmssd(intervals);

      // Smooth moving average sample signal
      final signal = Float64List.fromList([1.0, 3.0, 5.0, 3.0, 1.0]);
      final peaks = await Lamina.signal.findPeaks(signal);

      setState(() {
        _rmssd = rmssd;
        _peakCount = peaks.length;
        _status = 'Lamina Rust Engine Initialized Successfully';
      });
    } catch (e) {
      setState(() {
        _status = 'Error: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    const textStyle = TextStyle(fontSize: 18);
    const spacer = SizedBox(height: 12);
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(title: const Text('Lamina SDK Example')),
        body: Padding(
          padding: const EdgeInsets.all(16.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                _status,
                style: const TextStyle(
                  fontSize: 20,
                  fontWeight: FontWeight.bold,
                ),
              ),
              spacer,
              Text(
                'RMSSD (HRV): ${_rmssd != null ? "${_rmssd!.toStringAsFixed(2)} ms" : "Processing..."}',
                style: textStyle,
              ),
              spacer,
              Text('Peaks Detected: $_peakCount', style: textStyle),
            ],
          ),
        ),
      ),
    );
  }
}
