"""Multimodal feature extraction module for Lamina."""

import lamina._lamina as _native

WindowConfig = _native.PyWindowConfig
FeatureConfig = _native.PyFeatureConfig
MultimodalInput = _native.PyMultimodalInput
MultimodalFeatureVector = _native.PyMultimodalFeatureVector
extract_features = _native.extract_features
