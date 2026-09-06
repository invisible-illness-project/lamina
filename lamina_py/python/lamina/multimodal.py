"""Multimodal analysis, RSA, phase coupling, and quality assessment module for Lamina."""

from typing import Optional, List
import lamina._lamina as _native

RsaResult = _native.PyRsaResult
PhaseCouplingResult = _native.PyPhaseCouplingResult
ModalityQuality = _native.PyModalityQuality
MultimodalQuality = _native.PyMultimodalQuality

rsa = _native.rsa
cardiorespiratory_phase_coupling = _native.cardiorespiratory_phase_coupling
multimodal_quality = _native.multimodal_quality
