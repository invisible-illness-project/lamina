"""Remote photoplethysmography (rPPG) video processing module for Lamina."""

import lamina._lamina as _native

VideoFrame = _native.PyVideoFrame
VideoStream = _native.PyVideoStream
Roi = _native.PyRoi
RppgAlgorithmId = _native.PyRppgAlgorithmId
SignalPolarity = _native.PySignalPolarity
RppgWindowConfig = _native.PyRppgWindowConfig
RppgPreprocessingConfig = _native.PyRppgPreprocessingConfig
RppgConfig = _native.PyRppgConfig
RppgSegmentQuality = _native.PyRppgSegmentQuality
RppgSignal = _native.PyRppgSignal
extract_rppg = _native.extract_rppg
