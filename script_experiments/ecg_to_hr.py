import os
import sys
import argparse

import numpy as np
import pandas as pd
import neurokit2 as nk

import matplotlib.pyplot as plt

from tqdm import tqdm
from datetime import datetime

from adtk.detector import AutoregressionAD, ThresholdAD

script_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.append(os.path.join(script_dir, '../signal'))

from data_processing import DataProcessing

def load_dataset(script_dir, dataset_path, filter_by_subjects: list):
    """Load dataset from file path."""
    print("\n" + "="*50)
    print("LOAD DATASET")
    print("="*50)
    
    if not os.path.isabs(dataset_path):
        data_path = os.path.join(script_dir, dataset_path)
    else:
        data_path = dataset_path

    print(f"Dataset path: {data_path}")
    
    df = DataProcessing.load_from_file(data_path, file_type='csv', index_col=0, sep=',')
    # if 'Unnamed: 0' in df.columns:
    #     df.drop(['Unnamed: 0'], axis=1, inplace=True)

    if filter_by_subjects is None:
        print(f"Shape: {df.shape}")
        print(f"\nPreview:\n{df.head(7)}")
        print(f"\nPreview:\n{df.tail(7)}\n")
        print(f"\nPreview Labels and Count:\n{df['label'].value_counts()}\n")
        return df
    elif filter_by_subjects: 
        filt_subj = (df['subject'] == filter_by_subjects)
        subjects_df = df[filt_subj]
        print(f"Shape: {subjects_df.shape}")
        print(f"\nPreview {filter_by_subjects}:\n{subjects_df.head(7)}")
        print(f"\nPreview {filter_by_subjects}:\n{subjects_df.tail(7)}\n")
        print(f"\nPreview Labels and Count:\n{subjects_df['label'].values}\n")
        return subjects_df

def separate_subjects(df):
    subject_ids = df['subject'].unique()
    # print(labels)

    for subject_id in subject_ids:
        s_df = DataProcessing.get_specific_subject_df(df, subject_id)
        order = DataProcessing.check_labels_order(s_df)

        if np.array_equal(order, [3, 4]):
            print(f"Normal: {order} --- {subject_id}")
            df.loc[df['subject'] == subject_id, "Labels Order"] = "Normal"

        if np.array_equal(order, [4, 3, 4]):
            print(f"Abnormal: {order} --- {subject_id}")
            df.loc[df['subject'] == subject_id, "Labels Order"] = "Abnormal"
    
    return df

def _get_label_name(label_val):

    labels = {
        "1": "Baseline", 
        "2": "Stress",
        "3": "Amusement",
        "4": "Meditation"
    }
    return labels.get(str(label_val))

def convert_to_time(df, sampling_rate):

    timed_subject_dfs = []

    subject_ids = df['subject'].unique()
    labels = sorted(df['label'].unique())

    for subject_id in subject_ids:
        specifc_subject_dfs = []
        specific_subject_df = DataProcessing.get_specific_subject_df(df, subject_id)
        for label in labels:
            filt_labels = (specific_subject_df['label'] == label)
            subject_label_df = specific_subject_df[filt_labels]
            subject_label_df = subject_label_df.reset_index(drop=True)

            timed_subject_df = DataProcessing.create_time_df(subject_label_df, sampling_rate=sampling_rate)
            if subject_id == "S4":
                print("\n" + "="*60)
                print(f"Creating time component for subject {subject_id} --- label {label}")
                print("="*60)

                print(f"Shape: {timed_subject_df.shape}")
                print(f"\nPreview:\n{timed_subject_df[timed_subject_df['subject'] == "S4"].head(7)}")
                print(f"\nPreview:\n{timed_subject_df[timed_subject_df['subject'] == "S4"].tail(7)}\n")
            # print(f"\nPreview:\n{timed_subject_df.head(7)}")
            # print(f"\nPreview:\n{timed_subject_df.tail(7)}\n")
            specifc_subject_dfs.append(timed_subject_df)
    timed_subject_dfs.extend(specifc_subject_dfs)


    timed_subject_label_df = pd.concat(timed_subject_dfs, ignore_index=True)
    print(f"\nPreview:\n{timed_subject_label_df.head(7)}")
    print(f"\nPreview:\n{timed_subject_label_df.tail(7)}\n")
    
    return timed_subject_label_df

def split_into_segments(time_dfs, window_size):
    # print(window_size)
    segment_dfs = []

    labels = time_dfs['label'].unique()

    for label in labels:
        filt_label = (time_dfs['label'] == label)
        label_df = time_dfs[filt_label]

        # print(label_df.info())
        segmented_df, samples_per_window = DataProcessing.get_segments_by_duration(label_df, window_size)
        min_window_size = window_size // 60
        if label == 1:
            print("\n" + "="*60)
            print(f"[Segmenting label {label} into {window_size} seconds = {min_window_size}]")
            print("="*60)
            print(f"Shape: {segmented_df.shape}")
            print(f"\nPreview:\n{segmented_df.head(7)}\n")
            print(f"\nSamples per window:\n{samples_per_window}\n")
            print(f"\nPreview Labels and Count:\n{segmented_df['label'].value_counts()}\n")


            # ==========================
            # 4.3 VISUALIZE (POST-SEGMENTS)
            # ==========================
            plot_per_label(
                df=segmented_df, 
                s_id=args.subjects,
                pre_title="Post-Segmenting"
                )

        segment_dfs.append(segmented_df)

    segmented_by_label_df = pd.concat(segment_dfs, ignore_index=True)
    print(f"\nPreview:\n{segmented_by_label_df.head(7)}")
    print(f"\nPreview:\n{segmented_by_label_df.tail(7)}\n")

    return segmented_by_label_df

def get_r_peaks(segments_df, sampling_rate):

    windows_df = segments_df[['window_id', 'window_start_seconds', 'window_end_seconds']].drop_duplicates()
    windows_df = windows_df.sort_values('window_id').reset_index(drop=True)

    # Choose the signal column
    signal_col = 'Cleaned ECG' if 'Cleaned ECG' in segments_df.columns else 'ECG'

    # For fast slicing, use a sorted numeric seconds array once
    seconds = segments_df['Seconds'].to_numpy()
    print(seconds)
    signal  = segments_df[signal_col].to_numpy()
    print(len(signal))

    r_peaks = []

    for _, segment in tqdm(windows_df.iterrows(), total=len(windows_df)):
        start_time = float(segment['window_start_seconds'])
        end_time   = float(segment['window_end_seconds'])
        segment_idx =  segment['window_id']
        # print(f"Segment ({segment_idx})\n start time: {start_time} | end time: {end_time}")
        
        # Use the time window to get the index
        start_index = seconds.searchsorted(start_time, side='left')
        end_index   = seconds.searchsorted(end_time,   side='left')
        # print(f" start idx: {start_index} | end idx: {end_index}")

        # Use the indicies to get the signal's segment
        segment_ecg = signal[start_index:end_index]
        # print(f" segment idx: ({start_index} : {end_index}) | segment length: {len(segment_ecg)}\n\tsegment: {segment_ecg}")

        if segment_idx < 3:
            print(f"Segment ({segment_idx})\n start time: {start_time} | end time: {end_time}")
            print(f" start idx: {start_index} | end idx: {end_index}")
            print(f" segment idx: ({start_index} : {end_index}) | segment length: {len(segment_ecg)}\n\tsegment values: {segment_ecg}")

        # If segment is empty
        if segment_ecg.size == 0:
            r_peaks.append({
                'window_id': int(segment['window_id']),
                'window_start_seconds': start_time,
                'window_end_seconds': end_time,
                'n_samples': 0,
                'n_r_peaks': 0,
                'r_peaks_samples': np.array([], dtype=int),
                'note': 'empty window',
            })
            continue

        # If segment is NOT empty
        info = nk.ecg_findpeaks(segment_ecg, sampling_rate=sampling_rate)
        peaks = info.get('ECG_R_Peaks', np.array([], dtype=int))

        if segment_idx < 3:
            print(f"\t#peaks: {len(peaks)}")

        r_peaks.append({
            'window_id': int(segment['window_id']),
            'window_start_seconds': start_time,
            'window_end_seconds': end_time,
            'n_samples': int(segment_ecg.size),
            'n_r_peaks': int(peaks.size),
            'r_peaks_samples': peaks,  # indices are relative to the segment
        })

    return r_peaks

def get_hrv(r_peaks, sampling_rate):
    hrv_stats = []
    for r_peaks_idx in range(len(r_peaks)):
        r_peaks_info = r_peaks[r_peaks_idx]
        r_preak_name = r_peaks_info['window_id']
        r_peaks_per_segment = r_peaks_info['r_peaks_samples']

        # Add this check:
        # We need at least 2 peaks to compute 1 R-R interval.
        if len(r_peaks_per_segment) < 2:
            print(f"Skipping window {r_preak_name}: Not enough R-peaks found ({len(r_peaks_per_segment)}).")
            continue  # Skip to the next segment

        # computes time-domain indices of Heart Rate Variability (HRV)
        hrv_segment_stats = nk.hrv_time(r_peaks_per_segment, sampling_rate, show=False)
        
        # Add window_id to the stats for easy tracking
        hrv_segment_stats['window_id'] = r_preak_name
        
        if r_peaks_idx < 2:
            print(f"R-peaks segment ({r_peaks_idx}): {r_peaks_per_segment}")
            print(f"\tstats: {hrv_segment_stats.to_dict()}\n")
            
        hrv_stats.append(hrv_segment_stats)

    if not hrv_stats:
        print("No valid HRV stats were computed.")
        return pd.DataFrame()

    segment_stats_df = pd.concat(hrv_stats)
    segment_stats_df.reset_index(drop=True, inplace=True) # Use drop=True
    print("\n" + "="*60)
    print(f"HRV Stats per segment")
    print("="*60)
    print(f"Shape: {segment_stats_df.shape}")
    print(f"\nPreview:\n{segment_stats_df.head(7)}")
    print(f"\nPreview:\n{segment_stats_df.tail(7)}\n")
    return segment_stats_df

# Create a visualization class
def plot_per_label(df, s_id, pre_title):
    labels = sorted(df['label'].unique())
    fig, axes = plt.subplots(len(labels), 1, figsize=(14, 4 * len(labels)), sharex=False)
    if len(labels) == 1:
        axes = [axes]

    for ax, label in zip(axes, labels):
        filt_label = (df['label'] == label)
        label_df = df[filt_label]
        label_name = _get_label_name(label)

        # Plot the ECG signal
        ax.plot(label_df['Seconds'], label_df['Cleaned ECG'], linewidth=0.5, label='ECG')

        # Get the unique end times for each segment
        segment_ends = label_df['window_end_seconds'].unique()

        # Draw a vertical line at the end of each segment
        for end_time in segment_ends:
            ax.axvline(x=end_time, color='r', linestyle='--', linewidth=1)

        ax.set_title(f"{pre_title} --- {s_id} --- Label {label} ({label_name})")
        ax.set_xlabel("Seconds")
        ax.set_ylabel("Cleaned ECG")

    plt.tight_layout()
    plt.show()

if __name__ == "__main__":
    print("\n" + "="*60)
    print("ECG TO HR PIPELINE")
    print("="*50)
    
    # ============================================================
    # 1. CONFIGURATION
    # ============================================================
    base_data_path = DataProcessing.load_base_data_path(script_dir)
    print(base_data_path)
    default_dataset = os.path.join(base_data_path, 'WESAD/all_subjects/ecg_chest-subset_labels-cleaned.csv')
    default_save_path = os.path.join(base_data_path, 'ecg_to_hr')
    default_output_name = 'data.csv'
    default_sampling_rate = 700
    default_ecg_col_name = 'ECG'

    parser = argparse.ArgumentParser(
        description='Convert from ecg to hr.'
    )
    
    # dataset
    parser.add_argument(
        '--dataset', 
        default=default_dataset, 
        help='Path to dataset file'
        )
    
    # save_path
    parser.add_argument(
        '--save_path', 
        default=default_save_path, 
        help='Directory to save results'
        )
    
    # output_name
    parser.add_argument(
        '--output_name',
        default=default_output_name,
        help='Name for output file (e.g., )'
    )
    
    # subjects
    parser.add_argument(
        "--subjects",
        default="S4",
        help="Use all subjects, or filter by specific subject."
    )

    # window_size
    parser.add_argument(
        "--window_size",
        type=int,
        help="How to segment data across time (e.g. 60 secs (1 min), 300 secs (5 mins), etc)"
    )
    args = parser.parse_args()
    
    os.makedirs(args.save_path, exist_ok=True)
    
    # ==========================
    # 2. LOAD DF + NORMAL (3 -> 4) AND ABNORMAL (4 -> 3 -> 4)
    # ==========================
    df = load_dataset(script_dir, args.dataset, args.subjects)
    
    normal_abnormal_df = separate_subjects(df)
    filt_normal = (normal_abnormal_df["Labels Order"] == "Normal")
    normal_df = normal_abnormal_df[filt_normal]
    print(f"Shape: {normal_df.shape}")
    print(f"\nPreview:\n{normal_df.head(7)}")
    print(f"\nPreview:\n{normal_df.tail(7)}\n")
    print(f"\nPreview Labels and Count:\n{normal_df['label'].values}\n")

    # ==========================
    # 3. CONVERTED TO TIME: SUBJECT | LABEL | T0 - TN
    # ==========================
    time_subject_dfs = convert_to_time(normal_df, default_sampling_rate)

    # ==========================
    # 4.1 VISUALIZE (PRE-SEGMENTS)
    # ==========================
    # plot_per_label(
    #     df=time_subject_dfs, 
    #     s_id=args.subjects,
    #     pre_title="Pre-Segmenting"
    #     )
    
    # ==========================
    # 4.2 SEGMENT
    # ==========================
    segment_dfs = split_into_segments(time_subject_dfs, args.window_size)

    # ==========================
    # 4.3 VISUALIZE (POST-SEGMENTS)
    # ==========================
    plot_per_label(
        df=segment_dfs, 
        s_id=args.subjects,
        pre_title="Post-Segmenting"
        )
    
    r_peaks = get_r_peaks(segment_dfs, default_sampling_rate)
    hrv_df = get_hrv(r_peaks, default_sampling_rate)
