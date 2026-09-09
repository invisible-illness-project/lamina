# script_experiments/analyze_hrv.py
import os
import pandas as pd
import matplotlib.pyplot as plt
import argparse
from datetime import datetime


LABELS = ['baseline', 'stress', 'amusement', 'meditation']


def resolve_subject_dir(input_dir, subject_id):
    """
    Resolve subject directory on disk.
    Accepts: 13, S13, s13 → resolves to 'S13'
    """
    subject_id = str(subject_id).strip()

    # Normalize candidate names
    candidates = {
        subject_id,
        subject_id.upper(),
        f"S{subject_id.lstrip('S').lstrip('s')}"
    }

    for d in os.listdir(input_dir):
        if d in candidates:
            return d

    return None


def load_subject_data(input_dir, subject_id):
    """Loads all condition data for a single subject into a dictionary."""
    data_per_label = {}

    subject_dir_name = resolve_subject_dir(input_dir, subject_id)
    if subject_dir_name is None:
        print(f"❌ No directory found for subject '{subject_id}' in {input_dir}")
        return {}

    subject_dir = os.path.join(input_dir, subject_dir_name)

    print(f"--- Loading data for Subject {subject_dir_name} ---")

    for label in LABELS:
        label_folder = os.path.join(subject_dir, label)
        if not os.path.isdir(label_folder):
            print(f"  → Missing folder: {label}")
            continue

        subject_files = [
            f for f in os.listdir(label_folder)
            if f.startswith(subject_dir_name) and f.endswith('.csv')
        ]

        if not subject_files:
            print(f"  → No CSV found for label '{label}'")
            continue

        latest_file = sorted(subject_files)[-1]
        file_path = os.path.join(label_folder, latest_file)

        data_per_label[label] = pd.read_csv(file_path)
        print(f"  ✓ Loaded '{label}' from: {latest_file}")

    return data_per_label


def plot_subject_comparison(data_per_label, subject_dir_name, save_root):
    """Creates the overlay RMSSD comparison plot for a single subject."""
    print("\n--- Generating Comparison Plot ---")

    rmssd_col = 'HRV_RMSSD'
    plt.figure(figsize=(12, 7))

    for label, df in data_per_label.items():
        if rmssd_col in df.columns:
            plt.plot(
                df['window_id'],
                df[rmssd_col],
                marker='o',
                linestyle='-',
                label=label.title()
            )

    plt.title(f'RMSSD Comparison Across Conditions — {subject_dir_name}', fontsize=16)
    plt.xlabel('Segment Window ID', fontsize=12)
    plt.ylabel('RMSSD (ms)', fontsize=12)
    plt.grid(True, linestyle='--', alpha=0.7)
    plt.legend(fontsize=11)

    analysis_dir = os.path.join(save_root, subject_dir_name, 'analysis_plots')
    os.makedirs(analysis_dir, exist_ok=True)

    filename = f"{subject_dir_name}_RMSSD_Comparison_{datetime.now().strftime('%Y-%m-%d')}.png"
    save_path = os.path.join(analysis_dir, filename)

    plt.savefig(save_path)
    plt.close()

    print(f"✅ Comparison plot saved to: {save_path}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Analyze and plot subject-level HRV RMSSD comparison."
    )
    parser.add_argument(
        "--subject",
        required=True,
        help="Subject ID (e.g., 13 or S13)"
    )
    parser.add_argument(
        "--input_dir",
        default='../../data/ecg_to_hr',
        help="Base directory of processed HRV data"
    )

    args = parser.parse_args()

    subject_data = load_subject_data(args.input_dir, args.subject)

    if subject_data:
        subject_dir_name = resolve_subject_dir(args.input_dir, args.subject)
        plot_subject_comparison(subject_data, subject_dir_name, args.input_dir)
    else:
        print("⚠️ No data loaded. Plot not generated.")