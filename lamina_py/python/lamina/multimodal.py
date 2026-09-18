"""Multimodal analysis, RSA, phase coupling, and quality assessment module for Lamina."""

import lamina._lamina as _native

RsaConfig = _native.PyRsaConfig
PulseTimingConfig = _native.PyPulseTimingConfig
RsaResult = _native.PyRsaResult
PhaseCouplingResult = _native.PyPhaseCouplingResult
PulseTimingResult = _native.PyPulseTimingResult
CardiacRespiratoryEvent = _native.PyCardiacRespiratoryEvent
ScrCardiorespiratoryAssociation = _native.PyScrCardiorespiratoryAssociation
ModalityQuality = _native.PyModalityQuality
MultimodalQuality = _native.PyMultimodalQuality

rsa = _native.rsa
rsa_config = _native.rsa_config
cardiac_respiratory_phase = _native.cardiac_respiratory_phase
cardiorespiratory_phase_coupling = _native.cardiorespiratory_phase_coupling
ecg_ppg_timing = _native.ecg_ppg_timing
eda_cardiorespiratory_association = _native.eda_cardiorespiratory_association
respiratory_phase_at_time = _native.respiratory_phase_at_time
evaluate_ecg_quality = _native.evaluate_ecg_quality
evaluate_rsp_quality = _native.evaluate_rsp_quality
multimodal_quality = _native.multimodal_quality
