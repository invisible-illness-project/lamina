"""Multimodal feature extraction module for Lamina."""

from typing import Optional, List
import lamina._lamina as _native

FeatureConfig = _native.PyFeatureConfig
MultimodalInput = _native.PyMultimodalInput
MultimodalFeatureVector = _native.PyMultimodalFeatureVector
extract_features = _native.extract_features
