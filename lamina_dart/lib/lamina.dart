import 'dart:io';

import 'src/rust/api/autonomic.dart' as autonomic_api;
import 'src/rust/api/complexity.dart' as complexity_api;
import 'src/rust/api/ecg.dart' as ecg_api;
import 'src/rust/api/eda.dart' as eda_api;
import 'src/rust/api/features.dart' as features_api;
import 'src/rust/api/hrv.dart' as hrv_api;
import 'src/rust/api/multimodal.dart' as multimodal_api;
import 'src/rust/api/ppg.dart' as ppg_api;
import 'src/rust/api/rppg.dart' as rppg_api;
import 'src/rust/api/rsp.dart' as rsp_api;
import 'src/rust/api/signal.dart' as signal_api;
import 'src/rust/frb_generated.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

export 'src/rust/api/autonomic.dart'
    show
        AutonomicEstimator,
        AutonomicState,
        CardiacState,
        CouplingState,
        ElectrodermalState,
        RespiratoryState;
export 'src/rust/api/ecg.dart' show EcgPeakDetectionConfig;
export 'src/rust/api/eda.dart'
    show
        EdaComponentSignals,
        EdaDecompositionConfig,
        EdaPeakDetectionConfig,
        EdaPeakEvent;
export 'src/rust/api/error.dart' show SignalError;
export 'src/rust/api/features.dart'
    show CardiacFeatures, EdaFeatures, FeatureWindow, RespirationFeatures;
export 'src/rust/api/multimodal.dart'
    show PhaseCouplingResult, PulseTimingConfig, PulseTimingResult, RsaResult;
export 'src/rust/api/ppg.dart' show PpgPeakDetectionConfig;
export 'src/rust/api/rppg.dart'
    show RppgAlgorithmId, RppgConfig, RppgSignalResult;
export 'src/rust/api/rsp.dart' show RespirationCycle, RspProcessingConfig;
export 'src/rust/api/signal.dart' show PeakDetectionConfig;

/// Primary entry point for the Lamina physiological signal processing SDK.
class Lamina {
  /// Initializes the Lamina Rust backend via flutter_rust_bridge.
  static Future<void> init({String? dylibPath}) async {
    ExternalLibrary? lib;
    if (dylibPath != null) {
      lib = ExternalLibrary.open(dylibPath);
    } else {
      final candidates = [
        'librust_lib_lamina_dart.so',
        'target/debug/librust_lib_lamina_dart.so',
        'rust/target/debug/librust_lib_lamina_dart.so',
        '../target/debug/librust_lib_lamina_dart.so',
      ];
      for (final candidate in candidates) {
        final file = File(candidate);
        if (file.existsSync()) {
          lib = ExternalLibrary.open(file.absolute.path);
          break;
        }
      }
    }
    await RustLib.init(externalLibrary: lib);
  }

  static final signal = LaminaSignal();
  static final ecg = LaminaEcg();
  static final ppg = LaminaPpg();
  static final eda = LaminaEda();
  static final rsp = LaminaRsp();
  static final hrv = LaminaHrv();
  static final complexity = LaminaComplexity();
  static final features = LaminaFeatures();
  static final multimodal = LaminaMultimodal();
  static final autonomic = LaminaAutonomic();
  static final rppg = LaminaRppg();
}

class LaminaSignal {
  Future<Float64List> smoothMovingAverage(
    Float64List signal,
    int windowSize,
  ) async {
    final res = await signal_api.processSignalSmoothMovingAverage(
      signal: signal,
      windowSize: windowSize,
    );
    return Float64List.fromList(res);
  }

  Future<Float64List> filter(
    Float64List signal,
    double samplingRate,
    int order, {
    double? lowcut,
    double? highcut,
  }) async {
    final res = await signal_api.processSignalFilter(
      signal: signal,
      samplingRate: samplingRate,
      lowcut: lowcut,
      highcut: highcut,
      order: order,
    );
    return Float64List.fromList(res);
  }

  Future<List<int>> findPeaks(
    Float64List signal, {
    signal_api.PeakDetectionConfig? config,
  }) async {
    return await signal_api.processSignalFindpeaksConfig(
      signal: signal,
      config: config ?? const signal_api.PeakDetectionConfig(),
    );
  }
}

class LaminaEcg {
  Future<Float64List> clean(
    Float64List signal,
    double samplingRate, {
    String method = "neurokit",
  }) async {
    final res = await ecg_api.processEcgClean(
      signal: signal,
      samplingRate: samplingRate,
      method: method,
    );
    return Float64List.fromList(res);
  }

  Future<List<int>> findPeaks(
    Float64List signal,
    double samplingRate, {
    ecg_api.EcgPeakDetectionConfig? config,
  }) async {
    return await ecg_api.processEcgFindpeaks(
      signal: signal,
      samplingRate: samplingRate,
      config: config,
    );
  }

  Future<Uint8List> findPeaksMask(
    Float64List signal,
    double samplingRate,
  ) async {
    final res = await ecg_api.processEcgFindpeaksMask(
      signal: signal,
      samplingRate: samplingRate,
    );
    return Uint8List.fromList(res);
  }
}

class LaminaPpg {
  Future<Float64List> clean(Float64List signal, double samplingRate) async {
    final res = await ppg_api.processPpgClean(
      signal: signal,
      samplingRate: samplingRate,
    );
    return Float64List.fromList(res);
  }

  Future<List<int>> findPeaks(
    Float64List signal,
    double samplingRate, {
    ppg_api.PpgPeakDetectionConfig? config,
  }) async {
    return await ppg_api.processPpgFindpeaks(
      signal: signal,
      samplingRate: samplingRate,
      config: config,
    );
  }

  Future<Uint8List> findPeaksMask(
    Float64List signal,
    double samplingRate,
  ) async {
    final res = await ppg_api.processPpgFindpeaksMask(
      signal: signal,
      samplingRate: samplingRate,
    );
    return Uint8List.fromList(res);
  }
}

class LaminaEda {
  Future<Float64List> clean(Float64List signal, double samplingRate) async {
    final res = await eda_api.processEdaClean(
      signal: signal,
      samplingRate: samplingRate,
    );
    return Float64List.fromList(res);
  }

  Future<Float64List> phasic(Float64List signal, double samplingRate) async {
    final res = await eda_api.processEdaPhasic(
      signal: signal,
      samplingRate: samplingRate,
    );
    return Float64List.fromList(res);
  }

  Future<eda_api.EdaComponentSignals> decompose(
    Float64List signal,
    double samplingRate, {
    eda_api.EdaDecompositionConfig? config,
  }) async {
    return await eda_api.processEdaDecompose(
      signal: signal,
      samplingRate: samplingRate,
      config: config,
    );
  }

  Future<List<int>> findPeaks(
    Float64List phasicSignal,
    double samplingRate, {
    eda_api.EdaPeakDetectionConfig? config,
  }) async {
    return await eda_api.processEdaFindpeaks(
      phasicSignal: phasicSignal,
      samplingRate: samplingRate,
      config: config,
    );
  }

  Future<List<eda_api.EdaPeakEvent>> findPeaksEvents(
    Float64List phasicSignal,
    double samplingRate, {
    eda_api.EdaPeakDetectionConfig? config,
  }) async {
    return await eda_api.processEdaFindpeaksEvents(
      phasicSignal: phasicSignal,
      samplingRate: samplingRate,
      config: config,
    );
  }

  Future<Uint8List> findPeaksMask(
    Float64List phasicSignal,
    double samplingRate,
  ) async {
    final res = await eda_api.processEdaFindpeaksMask(
      phasicSignal: phasicSignal,
      samplingRate: samplingRate,
    );
    return Uint8List.fromList(res);
  }
}

class LaminaRsp {
  Future<Float64List> clean(Float64List signal, double samplingRate) async {
    final res = await rsp_api.processRspClean(
      signal: signal,
      samplingRate: samplingRate,
    );
    return Float64List.fromList(res);
  }

  Future<List<int>> findPeaks(
    Float64List cleanedSignal,
    double samplingRate, {
    rsp_api.RspProcessingConfig? config,
  }) async {
    return await rsp_api.processRspFindpeaks(
      cleanedSignal: cleanedSignal,
      samplingRate: samplingRate,
      config: config,
    );
  }

  Future<Uint8List> findPeaksMask(
    Float64List cleanedSignal,
    double samplingRate, {
    rsp_api.RspProcessingConfig? config,
  }) async {
    final res = await rsp_api.processRspFindpeaksMask(
      cleanedSignal: cleanedSignal,
      samplingRate: samplingRate,
      config: config,
    );
    return Uint8List.fromList(res);
  }

  Future<List<rsp_api.RespirationCycle>> cycles(
    Float64List cleanedSignal,
    double samplingRate, {
    rsp_api.RspProcessingConfig? config,
  }) async {
    return await rsp_api.processRspCycles(
      cleanedSignal: cleanedSignal,
      samplingRate: samplingRate,
      config: config,
    );
  }

  Future<Float64List> rate(
    Float64List cleanedSignal,
    double samplingRate, {
    rsp_api.RspProcessingConfig? config,
  }) async {
    final res = await rsp_api.processRspRate(
      cleanedSignal: cleanedSignal,
      samplingRate: samplingRate,
      config: config,
    );
    return Float64List.fromList(res);
  }
}

class LaminaHrv {
  Future<Float64List> peaksToIntervals(
    List<int> peaks,
    double samplingRate,
    int totalSamples,
  ) async {
    final res = await hrv_api.processPeaksToIntervals(
      peaks: peaks,
      samplingRate: samplingRate,
      totalSamples: totalSamples,
    );
    return Float64List.fromList(res);
  }

  Future<double> rmssd(Float64List intervals) async {
    return await hrv_api.processHrvRmssd(intervals: intervals);
  }

  Future<double> meanNn(Float64List intervals) async {
    return await hrv_api.processHrvMeanNn(intervals: intervals);
  }

  Future<double> sdnn(Float64List intervals) async {
    return await hrv_api.processHrvSdnn(intervals: intervals);
  }

  Future<double> pnn50(Float64List intervals) async {
    return await hrv_api.processHrvPnn50(intervals: intervals);
  }
}

class LaminaComplexity {
  Future<double> sampleEntropy(Float64List signal, int m, double r) async {
    return await complexity_api.processSampleEntropy(
      signal: signal,
      m: m,
      r: r,
    );
  }
}

class LaminaFeatures {
  Future<features_api.CardiacFeatures> extractCardiac(
    List<int> rPeaks,
    double samplingRate,
    double offsetSec,
    features_api.FeatureWindow window,
  ) async {
    return await features_api.extractCardiacFeatures(
      rPeaks: rPeaks,
      samplingRate: samplingRate,
      offsetSec: offsetSec,
      window: window,
    );
  }

  Future<features_api.EdaFeatures> extractEda(
    Float64List tonic,
    Float64List phasic,
    List<eda_api.EdaPeakEvent> events,
    double samplingRate,
    double offsetSec,
    features_api.FeatureWindow window,
  ) async {
    return await features_api.extractEdaFeatures(
      tonic: tonic,
      phasic: phasic,
      events: events,
      samplingRate: samplingRate,
      offsetSec: offsetSec,
      window: window,
    );
  }

  Future<features_api.RespirationFeatures> extractRespiration(
    List<rsp_api.RespirationCycle> cycles,
    double samplingRate,
    double offsetSec,
    features_api.FeatureWindow window,
  ) async {
    return await features_api.extractRespirationFeatures(
      cycles: cycles,
      samplingRate: samplingRate,
      offsetSec: offsetSec,
      window: window,
    );
  }
}

class LaminaMultimodal {
  Future<List<multimodal_api.PulseTimingResult>> pulseTiming(
    List<int> ecgPeaks,
    double ecgSamplingRate,
    double ecgOffsetSec,
    List<int> ppgPeaks,
    double ppgSamplingRate,
    double ppgOffsetSec, {
    multimodal_api.PulseTimingConfig? config,
  }) async {
    return await multimodal_api.computeEcgPpgTiming(
      ecgPeaks: ecgPeaks,
      ecgSamplingRate: ecgSamplingRate,
      ecgOffsetSec: ecgOffsetSec,
      ppgPeaks: ppgPeaks,
      ppgSamplingRate: ppgSamplingRate,
      ppgOffsetSec: ppgOffsetSec,
      config: config,
    );
  }

  Future<multimodal_api.PhaseCouplingResult> phaseCoupling(
    Float64List phases,
  ) async {
    return await multimodal_api.computeCardiorespiratoryPhaseCoupling(
      phases: phases,
    );
  }

  Future<multimodal_api.RsaResult> rsa(
    List<int> rPeaks,
    double ecgSamplingRate,
    double ecgOffsetSec,
    List<rsp_api.RespirationCycle> rspCycles,
    double rspSamplingRate,
    double rspOffsetSec,
  ) async {
    return await multimodal_api.computeRsa(
      rPeaks: rPeaks,
      ecgSamplingRate: ecgSamplingRate,
      ecgOffsetSec: ecgOffsetSec,
      rspCycles: rspCycles,
      rspSamplingRate: rspSamplingRate,
      rspOffsetSec: rspOffsetSec,
    );
  }
}

class LaminaAutonomic {
  Future<autonomic_api.AutonomicEstimator> createEstimator() async {
    return await autonomic_api.AutonomicEstimator.newInstance();
  }
}

class LaminaRppg {
  Future<rppg_api.RppgSignalResult> extractFromOpticalSignal(
    Float64List timestampsSec,
    Float64List red,
    Float64List green,
    Float64List blue,
    List<int> validPixels, {
    rppg_api.RppgConfig? config,
  }) async {
    return await rppg_api.extractRppgFromOpticalSignal(
      timestampsSec: timestampsSec,
      red: red,
      green: green,
      blue: blue,
      validPixels: validPixels,
      config: config,
    );
  }
}
