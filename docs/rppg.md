# Remote Photoplethysmography (rPPG) Substrate (`lamina::rppg`)

The `lamina::rppg` module provides a modular, deterministic, research-grade optical pulse signal-extraction substrate for Lamina. It transforms timestamped RGB video frame sequences and Region-of-Interest (ROI) tracking boundaries into standardized, quality-characterized camera-derived optical pulse surrogates (`RppgSignal`) paired with multi-tiered quality metadata (`RppgQualitySummary`).

---

## 1. Non-Clinical Guardrails & Scientific Rationale

> [!IMPORTANT]
> **Non-Clinical & Research Substrate Boundary**:
> The `lamina::rppg` module is an optical signal-processing substrate designed for physiological computing and research data analysis. It does **not** perform clinical vital-sign prediction, disease classification, neural model inference, SpO2/BP estimation, or automatic medical diagnosis. Downstream pulse interpretation (cleaning, peak detection, HRV) is explicitly decoupled and reuses Lamina's validated `lamina::ppg` pipeline.

### Scientific Scope & Principles
- **First-Class Quality Assessment**: Camera-derived pulse extraction performance is heavily influenced by motion, illumination, frame rate variability, and ROI bounds. rPPG output signals and quality metadata are inseparable.
- **Classical Deterministic Algorithms**: Implements foundational Green-channel baseline, CHROM (de Haan & Jeanne, 2013), and POS (Wang et al., 2017) methods operating over sliding temporal windows without neural network dependencies.
- **Downstream Decoupling**: Reuses Lamina's existing PPG feature pipeline (`lamina::ppg`) rather than creating a parallel feature-processing stack.

---

## 2. Scientific References and Architectural Basis

### Primary Scientific References

1. **Foundational rPPG Imaging (2008)**:
   Verkruysse W, Svaasand LO, Nelson JS. *Remote plethysmographic imaging using ambient light.* Optics Express. 2008;16(26):21434–21445.
   DOI: [10.1364/OE.16.021434](https://doi.org/10.1364/OE.16.021434) | PMID: [19104573](https://pubmed.ncbi.nlm.nih.gov/19104573/)

2. **Chrominance-Based rPPG / CHROM (2013)**:
   de Haan G, Jeanne V. *Robust pulse rate from chrominance-based rPPG.* IEEE Transactions on Biomedical Engineering. 2013;60(10):2878–2886.
   DOI: [10.1109/TBME.2013.2266196](https://doi.org/10.1109/TBME.2013.2266196) | PMID: [23744659](https://pubmed.ncbi.nlm.nih.gov/23744659/)

3. **Plane-Orthogonal-to-Skin / POS (2017)**:
   Wang W, den Brinker AC, Stuijk S, de Haan G. *Algorithmic Principles of Remote PPG.* IEEE Transactions on Biomedical Engineering. 2017;64(7):1479–1491.
   DOI: [10.1109/TBME.2016.2609282](https://doi.org/10.1109/TBME.2016.2609282) | PMID: [28113245](https://pubmed.ncbi.nlm.nih.gov/28113245/)

4. **Comparative CHROM/POS Evaluation (2023)**:
   *Contactless Cardiovascular Assessment by Imaging Photoplethysmography: A Comparison with Wearable Monitoring.* 2023.
   PMID: [36772543](https://pubmed.ncbi.nlm.nih.gov/36772543/)

5. **2026 rPPG Roadmap**:
   Elgendi M, Li S, Menon C, et al. *Roadmap of remote photoplethysmography from heart rate measurement toward clinical translation.* npj Digital Medicine. 2026;9:555.
   DOI: [10.1038/s41746-026-02715-1](https://doi.org/10.1038/s41746-026-02715-1) | PMID: [42106569](https://pubmed.ncbi.nlm.nih.gov/42106569/)

6. **Naturalistic Webcam Validation (2026)**:
   Woelk SP, Garfinkel SN, Mayiwar L, Knöferle K. *Advancing remote photoplethysmography (rPPG) to facilitate cardiac monitoring in naturalistic settings using webcam technology.* Behavior Research Methods. 2026;58(5):135.
   DOI: [10.3758/s13428-026-02953-x](https://doi.org/10.3758/s13428-026-02953-x) | PMID: [42026314](https://pubmed.ncbi.nlm.nih.gov/42026314/)

7. **rPPG Health-Assessment Review (2026)**:
   Brown T, Tulkens M, Mattelin B, Sanglet T, Dhuyvetters A. *Remote photoplethysmography for health assessment: a review informed by IntelliProve technology.* 2026.
   PMID: [41561164](https://pubmed.ncbi.nlm.nih.gov/41561164/)

8. **Challenging Illumination & Heart-Rate Conditions (2025)**:
   Acharya B, Saakyan W, Hammer B, Drimalla H. *The reliability of remote photoplethysmography under low illumination and elevated heart rates.* npj Digital Medicine. 2025;8(1):744.
   DOI: [10.1038/s41746-025-02192-y](https://doi.org/10.1038/s41746-025-02192-y) | PMID: [41339699](https://pubmed.ncbi.nlm.nih.gov/41339699/)

9. **Long-Duration Uncontrolled-Light Dataset (2025)**:
   *Evaluating remote photoplethysmography: A 10-minute video dataset in uncontrolled lighting.* 2025.
   PMID: [40747439](https://pubmed.ncbi.nlm.nih.gov/40747439/)

---

## 3. Decision-to-Reference Mappings

| Architectural Decision | Rationale | Primary Reference |
| :--- | :--- | :--- |
| **Decision A**: rPPG quality is a first-class output paired with waveform extraction. | Performance depends on motion, illumination, frame rate, and ROI quality. | Elgendi et al. (2026 Roadmap); Acharya et al. (2025) |
| **Decision B**: ROI and acquisition parameters are explicitly represented. | Facial/skin selection and pixel density heavily affect signal strength. | Verkruysse et al. (2008); 2026 rPPG Roadmap |
| **Decision C**: Classical algorithm abstraction supports interchangeable Green, CHROM, and POS methods. | CHROM and POS provide interpretable, robust classical baselines before learned models. | de Haan & Jeanne (2013); Wang et al. (2017); 2023 iPPG Review |
| **Decision D**: Staged HR-first validation strategy precedes beat-to-beat PRV claims. | Webcam rPPG demonstrates high accuracy for average HR, while beat-to-beat HRV remains challenging. | Woelk et al. (2026); 2023 iPPG Review |
| **Decision E**: Avoid higher-order health inference or disease prediction in rPPG layer. | Established evidence supports basic pulse estimation over higher-order health classification. | Brown et al. (2026) |
| **Decision F**: rPPG output integrates directly into existing `lamina::ppg` processing. | Standardized pulse waveforms should reuse validated peak detection and feature extraction. | 2022 PPG Review (PMID 35300400) |

---

## 4. Pipeline Architecture

```text
VideoStream (Frames + Physical Timestamps)
        ↓
ROI Provider (Static / Tracked ROI)
        ↓
ROI Sample Extraction (Spatial Mean RGB + Valid Pixel Counts)
        ↓
OpticalSignal (Temporal RGB Signals)
        ↓
Physical-Time Window Slicing [t_start, t_end) via Zero-Copy timestamp_range()
        ↓
Window-Local Preprocessing (Linear Detrending & Channel Mean Normalization)
        ↓
Interchangeable Algorithm (Green / CHROM / POS)
        ↓
Segment Quality Evaluation (ROI, Motion, Illumination, Signal)
        ↓
Quality-Weighted Overlap-Add Stitching & Gating (min_quality)
        ↓
RppgSignal & RppgQualitySummary (valid_fraction = valid_duration / total_duration)
        ↓
Gap-Aware Segment Extraction (RppgSignal::valid_segments(max_gap_sec))
        ↓
Downstream Integration (lamina::ppg::ppg_clean & ppg_findpeaks)
```

---

## 5. Mathematical Formulations

### 5.1 CHROM Algorithm (de Haan & Jeanne, 2013)
For normalized temporal channels $R_n = R / \mu_R - 1$, $G_n = G / \mu_G - 1$, $B_n = B / \mu_B - 1$:
$$X(t) = 3 R_n(t) - 2 G_n(t)$$
$$Y(t) = 1.5 R_n(t) + 1.5 G_n(t) - 3 B_n(t)$$
$$\alpha = \frac{\sigma_X}{\sigma_Y}$$
$$S_{\text{CHROM}}(t) = X(t) - \alpha Y(t)$$

### 5.2 POS Algorithm (Wang et al., 2017)
For normalized temporal channels $R_n = R / \mu_R$, $G_n = G / \mu_G$, $B_n = B / \mu_B$:
$$S_1(t) = G_n(t) - B_n(t)$$
$$S_2(t) = G_n(t) + B_n(t) - 2 R_n(t)$$
$$\alpha = \frac{\sigma_{S1}}{\sigma_{S2}}$$
$$P_{\text{POS}}(t) = S_1(t) + \alpha S_2(t)$$

### 5.3 Segment Quality Composite ($Q$)
$$Q = \frac{w_{\text{roi}} q_{\text{roi}} + w_{\text{motion}} q_{\text{motion}} + w_{\text{illum}} q_{\text{illum}} + w_{\text{signal}} q_{\text{signal}}}{w_{\text{roi}} + w_{\text{motion}} + w_{\text{illum}} + w_{\text{signal}}} \quad \in [0.0, 1.0]$$

Autocorrelation lag bounds for signal periodicity quality are derived from the configured physiological frequency band $[f_{\text{low}}, f_{\text{high}}]$:
$$\text{min\_lag} = \left\lceil \frac{F_s}{f_{\text{high}}} \right\rceil, \quad \text{max\_lag} = \left\lfloor \frac{F_s}{f_{\text{low}}} \right\rfloor$$

### 5.4 Recording Valid Duration Fraction
To prevent double-counting overlapping window hops, `valid_fraction` measures the total length of the single-pass forward merged union of valid segment intervals:
$$\text{valid\_fraction} = \frac{\text{duration}\left(\bigcup_i V_i\right)}{\text{total\_duration}} \quad \in [0.0, 1.0]$$

### 5.5 Piecewise Elementary Quality Integration
For contiguous valid segments extracted via `valid_segments()`, composite quality scores are computed by partitioning the segment interval into non-overlapping elementary sub-intervals $[\tau_k, \tau_{k+1})$, computing unweighted mean quality over simultaneously active windows $A_k$, and integrating over segment duration:
$$Q_{\text{segment}} = \frac{\sum_k \left( \frac{1}{|A_k|} \sum_{j \in A_k} Q_j \right) \cdot (\tau_{k+1} - \tau_k)}{t_B - t_A}$$

If an elementary sub-interval $[\tau_k, \tau_{k+1})$ has no active quality windows ($|A_k| = 0$), its contribution to all composite quality metrics is zero ($Q = 0.0$), ensuring that missing quality evidence cannot falsely inflate segment quality scores.

---

## 6. Algorithmic Complexity

| Stage | Complexity | Description |
| :--- | :---: | :--- |
| Frame ROI extraction | $O(N_{\text{frames}} \times \text{ROI}_{\text{pixels}})$ | Spatial RGB summation per frame |
| Physical timestamp range | $O(\log N_{\text{frames}})$ | Zero-copy binary search range determination |
| Window-local preprocessing | $O(L)$ | Linear detrending & channel normalization per window |
| CHROM / POS extraction | $O(W \times L)$ | Matrix projections over window length $L$ |
| Quality evaluation | $O(W \times L)$ | Discrete $\lceil F_s/f_{\text{high}} \rceil$ to $\lfloor F_s/f_{\text{low}} \rfloor$ autocorrelation |
| Overlap-add stitching | $O(N_{\text{frames}})$ | Weighted accumulation across timeline |
| Valid interval union | $O(V)$ | Single-pass forward merge of valid intervals |
| Valid segment extraction | $O(M)$ | Gap-aware non-NaN segment splitting with duration-weighted quality |
| Uniform resampling | $O(M)$ | Linear interpolation onto target sampling grid |
