import 'dart:typed_data';
import 'src/rust/api/simple.dart' as rust_api;
import 'src/rust/frb_generated.dart';

class Lamina {
  /// Initializes the lamina Rust backend via flutter_rust_bridge.
  static Future<void> init() async {
    await RustLib.init();
  }

  static final signal = LaminaSignal();
  static final ecg = LaminaEcg();
  static final ppg = LaminaPpg();
  static final eda = LaminaEda();
  static final rsp = LaminaRsp();
  static final hrv = LaminaHrv();
  static final complexity = LaminaComplexity();
}

class LaminaSignal {
  Future<Float64List> smoothMovingAverage(Float64List signal, int windowSize) async {
    return await rust_api.processSignalSmoothMovingAverage(signal: signal, windowSize: windowSize);
  }
  Future<Float64List> filter(Float64List signal, double samplingRate, int order, {double? lowcut, double? highcut}) async {
    return await rust_api.processSignalFilter(signal: signal, samplingRate: samplingRate, lowcut: lowcut, highcut: highcut, order: order);
  }
  Future<List<bool>> findPeaks(Float64List signal) async {
    return await rust_api.processSignalFindpeaks(signal: signal);
  }
}

class LaminaEcg {
  Future<Float64List> clean(Float64List signal, double samplingRate, {String method = "neurokit"}) async {
    return await rust_api.processEcgClean(signal: signal, samplingRate: samplingRate, method: method);
  }
  Future<List<bool>> findPeaks(Float64List signal, double samplingRate) async {
    return await rust_api.processEcgFindpeaks(signal: signal, samplingRate: samplingRate);
  }
}

class LaminaPpg {
  Future<Float64List> clean(Float64List signal, double samplingRate) async {
    return await rust_api.processPpgClean(signal: signal, samplingRate: samplingRate);
  }
  Future<List<bool>> findPeaks(Float64List signal, double samplingRate) async {
    return await rust_api.processPpgFindpeaks(signal: signal, samplingRate: samplingRate);
  }
}

class LaminaEda {
  Future<Float64List> clean(Float64List signal, double samplingRate) async {
    return await rust_api.processEdaClean(signal: signal, samplingRate: samplingRate);
  }
  Future<Float64List> phasic(Float64List signal, double samplingRate) async {
    return await rust_api.processEdaPhasic(signal: signal, samplingRate: samplingRate);
  }
  Future<List<bool>> findPeaks(Float64List phasicSignal) async {
    return await rust_api.processEdaFindpeaks(phasicSignal: phasicSignal);
  }
}

class LaminaRsp {
  Future<Float64List> clean(Float64List signal, double samplingRate) async {
    return await rust_api.processRspClean(signal: signal, samplingRate: samplingRate);
  }
  Future<List<bool>> findPeaks(Float64List cleanedSignal) async {
    return await rust_api.processRspFindpeaks(cleanedSignal: cleanedSignal);
  }
}

class LaminaHrv {
  Future<Float64List> peaksToIntervals(List<bool> peaks, double samplingRate) async {
    return await rust_api.processPeaksToIntervals(peaks: peaks, samplingRate: samplingRate);
  }
  Future<double?> rmssd(Float64List intervals) async {
    return await rust_api.processHrvRmssd(intervals: intervals);
  }
  Future<double?> meanNn(Float64List intervals) async {
    return await rust_api.processHrvMeanNn(intervals: intervals);
  }
}

class LaminaComplexity {
  Future<double> sampleEntropy(Float64List signal, int m, double r) async {
    return await rust_api.processSampleEntropy(signal: signal, m: m, r: r);
  }
}
