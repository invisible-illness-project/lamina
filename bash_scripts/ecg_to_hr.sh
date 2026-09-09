#!/usr/bin/env bash
# ecg_to_hr.sh
# Convert ECG signals to HR / HRV measurements

set -euo pipefail
cd ../script_experiments

WINDOW_SIZE=150
# SUBJECTS=(4 13)
SUBJECTS=(2 3 4 5 6 7 8 9 10 11 13 14 15 16 17)

echo "============================================================"
echo "       CONVERT FROM ECG TO HR / HRV MEASUREMENTS"
echo "============================================================"
echo ""

# =================================================================
# STAGE 1: ECG → HRV FEATURE EXTRACTION
# =================================================================
for subject in "${SUBJECTS[@]}"; do
    subject_id="S${subject}"

    echo "------------------------------------------------------------"
    echo "▶ Running experiments for subject: ${subject_id}"
    echo "------------------------------------------------------------"

    if ! python ecg_to_hr.py \
        --subjects "${subject_id}" \
        --window_size "${WINDOW_SIZE}"; then
        echo "⚠️  Warning: Failed to process subject ${subject_id}. Continuing."
    else
        echo "✔ Completed subject: ${subject_id}"
    fi

    echo ""
done

# =================================================================
# STAGE 2: FINAL ANALYSIS & COMPARISON PLOTTING
# =================================================================
echo ""
echo "============================================================"
echo "   STAGE 2: CREATING COMPARISON PLOTS FOR EACH SUBJECT"
echo "============================================================"

for subject in "${SUBJECTS[@]}"; do
    subject_id="S${subject}"

    echo "------------------------------------------------------------"
    echo "▶ Analyzing subject: ${subject_id}"

    if ! python analyze_hr.py --subject "${subject_id}"; then
        echo "⚠️  Warning: Failed to create comparison plot for ${subject_id}."
    else
        echo "✔ Created comparison plot for: ${subject_id}"
    fi
done

echo ""
echo "============================================================"
echo "✅ All processing and analysis complete."
echo "============================================================"