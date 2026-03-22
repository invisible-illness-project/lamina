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
            print("\n" + "="*60)
            print(f"Creating time component for subject {subject_id} --- label {label}")
            print("="*60)

            timed_subject_df = DataProcessing.create_time_df(subject_label_df, sampling_rate=sampling_rate)
            # if subject_id == "S4":
            #     print(f"Shape: {timed_subject_df.shape}")
            #     print(f"\nPreview:\n{timed_subject_df[timed_subject_df['subject'] == "S4"].head(7)}")
            #     print(f"\nPreview:\n{timed_subject_df[timed_subject_df['subject'] == "S4"].tail(7)}\n")
            print(f"\nPreview:\n{timed_subject_df.head(7)}")
            print(f"\nPreview:\n{timed_subject_df.tail(7)}\n")
            specifc_subject_dfs.append(timed_subject_df)
    timed_subject_dfs.append(specifc_subject_dfs)


    timed_subject_df = pd.DataFrame(timed_subject_dfs)
    
    return timed_subject_df

# def split_into_segments(time_dfs, sampling_rate):
#     segment_dfs = []
#     for idx, row in time_dfs.
#         print("\n" + "="*60)
#         print(f"[Segmenting label {label_val} into 5-minute windows]")
#         print("="*60)
#         segmented_df, samples_per_window = DataProcessing.get_segments_by_duration(timed_df, 300)
#         print(f"Shape: {segmented_df.shape}")
#         print(f"\nPreview:\n{segmented_df.head(7)}\n")
#         print(f"\nSamples per window:\n{samples_per_window}\n")
#         print(f"\nPreview Labels and Count:\n{segmented_df['label'].value_counts()}\n")

#         segment_dfs.append(segmented_df)

#     return segment_dfs

# Create a visualization class
# def plot_rmssd_per_label(segment_dfs):
#     for segmented_df in segment_dfs:
#         label_val = segmented_df['label'].iloc[0]
#         label_name = _get_label_name(label_val)
#         print("\n" + "="*60)
#         print(f"Plotting Segment for label: {label_val}---{label_name}")
#         print("="*60)

#         plt.figure(figsize=(12, 4))
#         plt.plot(segmented_df['Seconds'], segmented_df['ECG'], linewidth=0.5)
#         plt.title(f"ECG Signal — Label {label_val}")
#         plt.xlabel("Seconds")
#         plt.ylabel("ECG")
#         plt.tight_layout()
#         plt.show()

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
        default=None,
        help="Use all subjects, or filter by specific subject."
    )
    args = parser.parse_args()
    
    os.makedirs(args.save_path, exist_ok=True)
    
    df = load_dataset(script_dir, args.dataset, args.subjects)
    normal_abnormal_df = separate_subjects(df)

    filt_normal = (normal_abnormal_df["Labels Order"] == "Normal")
    normal_df = normal_abnormal_df[filt_normal]
    print(f"Shape: {normal_df.shape}")
    print(f"\nPreview:\n{normal_df.head(7)}")
    print(f"\nPreview:\n{normal_df.tail(7)}\n")
    print(f"\nPreview Labels and Count:\n{normal_df['label'].values}\n")

    get_all_labels = DataProcessing.get_all_labels(normal_df)


    time_subject_dfs = convert_to_time(normal_df, default_sampling_rate)
    # segment_dfs = split_into_segments(time_dfs, default_sampling_rate)
    # plot_rmssd_per_label(segment_dfs)
