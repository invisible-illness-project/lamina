import pytest
import numpy as np
import lamina

def test_rppg_extract():
    w, h = 64, 64
    frames = []
    for i in range(60):
        # Create a synthetic 64x64 RGB frame with pulsing green channel
        frame_data = np.zeros((h, w, 3), dtype=np.uint8)
        frame_data[:, :, 0] = 120
        frame_data[:, :, 1] = int(120 + 20 * np.sin(2 * np.pi * 1.2 * (i / 30.0)))
        frame_data[:, :, 2] = 100
        
        frame = lamina.rppg.VideoFrame(timestamp_sec=i / 30.0, width=w, height=h, data=frame_data.tobytes())
        frames.append(frame)

    stream = lamina.rppg.VideoStream(frames=frames, nominal_fps=30.0)
    roi = lamina.rppg.Roi(x=0, y=0, width=w, height=h)
    cfg = lamina.rppg.RppgConfig()
    result = lamina.rppg.extract_rppg(stream, roi, cfg)
    
    assert hasattr(result, "timestamps_sec")
    assert hasattr(result, "waveform")
    assert hasattr(result, "sampling_rate_hz")
    assert hasattr(result, "overall_quality")
