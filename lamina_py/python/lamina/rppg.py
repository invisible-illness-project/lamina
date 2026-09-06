"""Remote photoplethysmography (rPPG) video processing module for Lamina."""

from typing import Optional
import lamina._lamina as _native

VideoFrame = _native.PyVideoFrame
VideoStream = _native.PyVideoStream
Roi = _native.PyRoi
RppgConfig = _native.PyRppgConfig
RppgSignal = _native.PyRppgSignal
extract_rppg = _native.extract_rppg
