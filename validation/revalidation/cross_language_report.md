# Wave H — Cross-Language / Protobuf Validation Report

**Scope:** `sensor_messages` @ `support-lamina-upgrade` (ed24f455fed73f85cd5c318fbaf65cea022e7486) vs Lamina post-remediation (`/mnt/agents/lamina-reval`). Validation-only; neither repository was modified. Test harnesses were created outside the repos: `$HOME/waveh` (Rust bin, path deps), `$HOME/waveh_py.py` (Python), `$HOME/dart_test` (Dart package, path dep). Byte exchange via `/home/kimi/waveh_data`.

## Environment

| Component | Version |
|---|---|
| Rust | 1.98.1 stable (rsproxy mirror), prost/prost-build 0.13, protoc 25.6 (official binary) |
| Python | 3.12.12, protobuf runtime 6.32.1 (pb2 gencode 6.30.2 — compatible) |
| Dart | SDK 3.13.3 stable (dart-archive download), protobuf 6.0.0, fixnum 1.1.1 |

**All three runtime legs were executed for real — including Dart** (SDK downloaded successfully; no static-only fallback needed).

## What was tested

1. **Round-trip legs** (byte-level + semantic decode) for the messages most relevant to Lamina's new API: `BvpWaveform`, `RppgSignal` (nested `RppgSegment`/`RppgQualitySummary`), `ContinuousSignalBatch` (modality/unit enums, `missing_mask`, `FilterSpec`), `SensorDataContainer` (oneof payload incl. `bvp_waveform` tag 22, `google.protobuf.Timestamp`).
   - Rust→Python, Python→Rust, Dart→Rust, Rust→Dart, Python↔Dart (via byte identity with Rust canonical encodings + direct decode).
2. **Robustness**: unknown enum value (7), negative enum (-1), proto3 zero-vs-unset ambiguity, proto3 `optional` explicit presence, NaN/subnormal/extreme f64 preservation.
3. **Schema consistency**: field tag numbers/types across `.proto` ↔ prost-generated Rust ↔ generated Dart (217 fields parsed programmatically from all three); all 7 enums compared.
4. **Proto ↔ Lamina enum/type cross-check**: `SignalPolarity`, `IntervalQuality`, `CorrectionPolicy`, `BeatQuality`, `RppgAlgorithmId`, `BvpWaveform`/`RppgSignal`/`RppgSegment` field mapping.
5. **Repo's own test suites**: cargo test, python unittest, dart test — all pass (4/4, 3/3, 3/3), matching ARCHITECTURE_REPORT §7 claims.

## Results — round-trips

**All byte-level encodings are identical across Rust (prost), Python (protobuf 6.32), and Dart (protobuf 6.0).** Example `BvpWaveform` (125 bytes, identical in all three):

```
09 b4e60740fc54d941        field1 start_sec    = 1700000000.123456 (f64 LE)
11 1b81e85eb51f84d4 40     field2 sampling_rate_hz = 59.94 (bit-exact non-integer)
1a 30 ...                  field3 pulse_samples packed f64 x6 (incl. 4.94e-324 subnormal min-normal, 1e300)
20 01                      field4 polarity = NORMAL(1)
2a 35 ...                  field5 metadata (DeviceMetadata)
```

f64 values verified bit-exact (`to_bits()`/`struct.pack`/`ByteData` comparisons) in both directions for every leg. NaN payload bits survive all legs. No f32 anywhere: the `.proto` contains zero `float` fields and the prost binding contains zero `f32` — **no precision loss**.

**Enum transport**: all in-range enum values (SignalPolarity 0–3, IntervalQualityKind 0–4, CorrectionPolicyKind 0–5, SensorModality, PhysicalUnit, FilterKind, FeatureQualityIssue) round-trip identically in all three languages. Enum value sets are identical across the three generated bindings and the `.proto` (verified programmatically).

**Unknown enum values** (forward-compat probe, wire bytes `20 07` = polarity 7):
- Rust (prost): decodes, raw `i32=7` preserved in field; `SignalPolarity::try_from` → `UnknownEnumValue(7)`. No data loss.
- Python: decodes, `polarity == 7` readable (proto3 open enum); re-serializes to `2007`.
- **Dart: unknown value is diverted to `unknownFields`; the typed getter `polarity` returns the default `SIGNAL_POLARITY_UNSPECIFIED(0)`** — indistinguishable from a genuinely-zero field at the API level. Bytes are preserved on re-serialization (`writeToBuffer` → `2007`). Same behavior for negative enum value `-1`.

**Default-value ambiguity (proto3, expected)**: unset `polarity` and explicit `SIGNAL_POLARITY_UNSPECIFIED(0)` produce byte-identical wire data in Rust and Python (`110000000000003e40`). Dart's presence-tracked setters serialize an explicitly-assigned zero enum (`...3e40 2000`) — non-canonical but decode-equivalent everywhere. `optional double` fields (e.g. `CardiacFeatures.mean_hr_bpm`) carry explicit presence in all three bindings (set-to-0.0 distinguishable from unset; prost generates `Option<f64>`).

## Findings

### P1 — material (semantic data loss in proto ↔ Lamina mapping)

1. **`CorrectionPolicy::PercentThreshold(f64)` parameter cannot be transported.** Proto `CorrectionPolicyKind` (`sensor-messages.proto:85-92`) is a bare enum; Lamina `CorrectionPolicy::PercentThreshold(f64)` (`src/hrv/quality.rs:44`) carries the threshold. On the wire the f64 silently disappears — a receiver cannot reproduce the correction behavior.
2. **`RppgSignal.algorithm: RppgAlgorithmId` has no proto representation.** Lamina `RppgSignal` (`src/rppg/signal.rs:282-297`, field `algorithm` at :292) with `RppgAlgorithmId {GreenChannel, Chrom, Pos}` (`src/rppg/config.rs:5-13`); proto `RppgSignal` (`sensor-messages.proto:444-452`) has no algorithm field and no matching enum.
3. **Per-sample timestamps lost for rPPG/BVP waveforms.** Lamina `BvpWaveform.timestamps_sec: Vec<f64>` (`src/rppg/signal.rs:264-271`) and `RppgSegment.timestamps_sec` + `quality: RppgSegmentQuality` (`src/rppg/signal.rs:212-224`) carry irregular physical timestamps (essential for gappy camera-derived rPPG). Proto `BvpWaveform` (`:455-461`) and `RppgSegment` (`:436-441`) model only `start_sec` + `sampling_rate_hz` + samples, i.e. uniform sampling; non-uniform timing and per-segment quality do not round-trip. (Proto `RppgSignal.valid_segments` can partially recover segmentation, but not intra-segment timestamp jitter.)

### P2 — moderate

4. **`IntervalQualityKind` and `CorrectionPolicyKind` are dead schema**: defined in the proto (`:76-92`) but referenced by **no message field** (verified by grep over all message definitions). Interval quality classifications and correction policy cannot actually be transported at all today — the enums exist but are unreachable.
5. **Dart hides unknown enum values at the typed-API level** (returns default 0; raw value only in `unknownFields`). Rust/Python expose the raw value. Cross-language enum-forward-compat behavior is therefore asymmetric; a Dart consumer cannot detect "new enum variant arrived".
6. **Lamina `BeatQuality` {Normal, Ectopic, Artifact, Unknown}** (`src/hrv/quality.rs:6-16`) has no proto counterpart — beat-level quality cannot be transported (related to finding 4).

### P3 — minor

7. **Default semantics mismatch**: Lamina `SignalPolarity::default()` = `Normal` (runtime-verified); an absent proto `polarity` decodes to `UNSPECIFIED(0)`. Receivers must not equate "absent" with Lamina's default.
8. **Extra proto field without Lamina counterpart**: `RppgSignal.polarity` (tag 7) / `BvpWaveform.polarity` exist in proto, but in Lamina polarity is a conversion *parameter* (`to_bvp_waveform(polarity)`), not stored signal state. Forward-compatible, but semantically asymmetric.
9. **Documentation drift**: ARCHITECTURE_REPORT.md §4 oneof listing (`:92-106`) omits `bvp_waveform = 22` (added to the schema later); §5 compatibility matrix lists Python `sampling_rate_hz` as "`float`" (misleading — Python floats are 64-bit; no f32 anywhere in the stack).
10. **Cosmetic**: prost variant names for `IntervalQualityKind`/`CorrectionPolicyKind` are not prefix-stripped (`IntervalQualityNormalNn`, …) unlike the other enums, because proto value names don't carry the full enum-name prefix. No functional impact.

## What passed cleanly (no issues)

- All 6 runtime round-trip legs byte-identical; f64 bit-exactness incl. NaN, subnormals, 1e300; Timestamp nanos preserved (123456789, 500000000).
- 217/217 message field tag numbers consistent across `.proto`, prost Rust, and Dart bindings; oneof tags 10–22 consistent.
- All 7 enums have identical value sets in all three bindings and match Lamina variants 1:1 for `SignalPolarity` and `IntervalQuality`.
- `proto3 optional` presence semantics consistent (Rust `Option<f64>`, Python/Dart `HasField`).
- Repo test suites: Rust 4/4, Python 3/3, Dart 3/3 pass.

## Not tested / limitations

- JSON mappings (`*.pbjson.dart`, `toProto3Json`) not exercised — binary wire format only.
- Large-payload/packed-encoding stress and streaming partial-decode not tested.
- Go/Java bindings (options declared in proto) not generated or tested — out of scope of the three-language contract.
- TimescaleDB column mapping in ARCHITECTURE_REPORT §5 not validated (no DB available).
